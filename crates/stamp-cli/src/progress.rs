//! Progress of slow steps on standard error: a spinner, or a bar where the
//! work has a count (rows of a distance matrix, permutations, features).
//!
//! Shown only when standard error is a terminal, so that nothing is added
//! to pipes, logs or test output, and not with `--quiet`; and only for a
//! step still running after half a second, so that quick ones do not
//! flicker. A thread draws it a few times a second, so the work itself only
//! stores how far it has got.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Whether progress is shown at all.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Setting this variable shows progress even when standard error is not a
/// terminal, at once: for testing.
const FORCE_VARIABLE: &str = "STAMP_FORCE_PROGRESS";

/// How long a step runs before its progress is shown.
const DELAY: Duration = Duration::from_millis(500);

/// How often the progress is drawn.
const INTERVAL: Duration = Duration::from_millis(100);

/// Decide, once, whether progress is shown.
pub fn init(quiet: bool) {
    let shown = !quiet && (std::io::stderr().is_terminal() || forced());
    ENABLED.store(shown, Ordering::Relaxed);
}

fn forced() -> bool {
    std::env::var_os(FORCE_VARIABLE).is_some_and(|v| !v.is_empty())
}

#[derive(Default)]
struct State {
    stage: String,
    done: usize,
    total: usize,
}

/// A slow step whose progress is shown while it lasts.
pub struct Task {
    shared: Option<Shared>,
}

struct Shared {
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    drawer: Option<std::thread::JoinHandle<()>>,
}

impl Task {
    /// Start showing `stage` (after a moment), with no count.
    pub fn start(stage: &str) -> Task {
        if !ENABLED.load(Ordering::Relaxed) {
            return Task { shared: None };
        }
        let state = Arc::new(Mutex::new(State {
            stage: stage.to_string(),
            ..State::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let delay = if forced() { Duration::ZERO } else { DELAY };
        let drawer = {
            let (state, stop) = (Arc::clone(&state), Arc::clone(&stop));
            std::thread::spawn(move || draw_until_stopped(&state, &stop, delay))
        };
        Task {
            shared: Some(Shared {
                state,
                stop,
                drawer: Some(drawer),
            }),
        }
    }

    /// Say what is being done now and how far it has got (`total` zero
    /// when that cannot be told).
    pub fn set(&self, stage: &str, done: usize, total: usize) {
        if let Some(shared) = &self.shared {
            let mut state = shared.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.stage != stage {
                state.stage.clear();
                state.stage.push_str(stage);
            }
            state.done = done;
            state.total = total;
        }
    }

    /// How far the current stage has got.
    pub fn count(&self, done: usize, total: usize) {
        if let Some(shared) = &self.shared {
            let mut state = shared.state.lock().unwrap_or_else(|e| e.into_inner());
            state.done = done;
            state.total = total;
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        if let Some(mut shared) = self.shared.take() {
            shared.stop.store(true, Ordering::Relaxed);
            if let Some(drawer) = shared.drawer.take() {
                let _ = drawer.join();
            }
        }
    }
}

/// Draw the progress every so often from `delay` on, until told to stop;
/// then clear the line, if anything was drawn.
fn draw_until_stopped(state: &Mutex<State>, stop: &AtomicBool, delay: Duration) {
    let start = Instant::now();
    let mut drawn = false;
    let mut frame = 0usize;
    loop {
        // Checked after drawing, so that with no delay something is drawn
        // however quick the step.
        let stopping = stop.load(Ordering::Relaxed);
        if start.elapsed() >= delay {
            let line = {
                let state = state.lock().unwrap_or_else(|e| e.into_inner());
                line(&state, frame, start.elapsed())
            };
            let mut stderr = std::io::stderr().lock();
            let _ = write!(stderr, "\r\x1b[2K{line}");
            let _ = stderr.flush();
            drawn = true;
            frame += 1;
        }
        if stopping {
            break;
        }
        // Woken often enough to stop promptly.
        let mut slept = Duration::ZERO;
        while slept < INTERVAL && !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(10));
            slept += Duration::from_millis(10);
        }
    }
    if drawn {
        let mut stderr = std::io::stderr().lock();
        let _ = write!(stderr, "\r\x1b[2K");
        let _ = stderr.flush();
    }
}

/// One line of progress: a spinner and the stage, with a bar and the count
/// when there is one, and the time so far.
fn line(state: &State, frame: usize, elapsed: Duration) -> String {
    const SPINNER: [char; 4] = ['|', '/', '-', '\\'];
    const WIDTH: usize = 30;
    let spinner = SPINNER[frame % SPINNER.len()];
    let seconds = elapsed.as_secs();
    let time = format!("{}:{:02}", seconds / 60, seconds % 60);
    if state.total > 0 {
        let done = state.done.min(state.total);
        let filled = done * WIDTH / state.total;
        format!(
            "{spinner} {} [{}{}] {done}/{} {time}",
            state.stage,
            "#".repeat(filled),
            " ".repeat(WIDTH - filled),
            state.total
        )
    } else {
        format!("{spinner} {}... {time}", state.stage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_show_a_bar_when_there_is_a_count() {
        let state = State {
            stage: "PERMANOVA".to_string(),
            done: 412,
            total: 999,
        };
        assert_eq!(
            line(&state, 0, Duration::from_secs(75)),
            "| PERMANOVA [############                  ] 412/999 1:15"
        );
        let state = State {
            stage: "Reading profile.spf".to_string(),
            done: 0,
            total: 0,
        };
        assert_eq!(
            line(&state, 1, Duration::from_secs(2)),
            "/ Reading profile.spf... 0:02"
        );
    }

    #[test]
    fn nothing_happens_when_progress_is_not_shown() {
        // Tests run with standard error captured, not on a terminal.
        let task = Task { shared: None };
        task.set("Something", 1, 2);
        task.count(2, 2);
    }
}
