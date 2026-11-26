//! The long calculations behind the community plots (distances, NMDS, the
//! community tests, the Mantel test and ANCOM) can be carried out a piece
//! at a time: whatever runs them pauses between pieces to show progress or
//! to stop when cancelled. The results must not depend on
//! where they pause, and the progress they report must only go forwards.

use stamp_core::stats::ancom::{ancom, AncomRun};
use stamp_core::stats::community::{mantel, CommunityTest, MantelRun};
use stamp_core::stats::correlation::CorrelationMethod;
use stamp_core::stats::diversity::{distances, nmds, pcoa, Distance, DistanceRun, NmdsRun};
use stamp_core::stats::multi_group::MultiGroupTest;
use stamp_core::stats::results::{Control, Pace, Step};
use stamp_core::stats::rng_from_seed;

/// A table of `n` samples of `features` features, with some zeros, in
/// three groups that differ.
fn table(n: usize, features: usize) -> (Vec<Vec<f64>>, Vec<usize>) {
    let mut state: u64 = 11;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as f64 / (1u64 << 31) as f64
    };
    let groups: Vec<usize> = (0..n).map(|s| s % 3).collect();
    let samples = (0..n)
        .map(|s| {
            (0..features)
                .map(|f| {
                    let shift = if f % 4 == groups[s] { 5.0 } else { 1.0 };
                    let value = (next() * 30.0 * shift).floor();
                    if value < 4.0 {
                        0.0
                    } else {
                        value
                    }
                })
                .collect()
        })
        .collect();
    (samples, groups)
}

/// What a run reported while it was advanced to the end, pausing before
/// every `every`-th piece of work.
struct Stepped<T> {
    result: T,
    pauses: usize,
    reports: Vec<(usize, usize)>,
}

fn stepped<T>(every: usize, advance: &mut dyn FnMut(Pace<'_>) -> Step<T>) -> Stepped<T> {
    let mut pauses = 0;
    let mut reports = Vec::new();
    let mut calls = 0;
    loop {
        let step = advance(&mut |done, total| {
            reports.push((done, total));
            calls += 1;
            if calls % every == 0 {
                Control::Pause
            } else {
                Control::Continue
            }
        });
        match step {
            Step::Done(result) => {
                return Stepped {
                    result,
                    pauses,
                    reports,
                }
            }
            Step::Paused => pauses += 1,
            Step::Cancelled => panic!("cancelled"),
        }
        assert!(pauses < 100_000, "the run does not finish");
    }
}

/// Progress only goes forwards, against one total, and never beyond it.
fn assert_monotone(reports: &[(usize, usize)]) {
    assert!(!reports.is_empty());
    let total = reports[0].1;
    let mut last = 0;
    for &(done, t) in reports {
        assert_eq!(t, total);
        assert!(done >= last, "progress went back: {reports:?}");
        assert!(done <= total);
        last = done;
    }
}

#[test]
fn distances_are_the_same_wherever_they_pause() {
    let (samples, _) = table(40, 25);
    for metric in [Distance::BrayCurtis, Distance::Jaccard, Distance::Aitchison] {
        let whole = distances(&samples, metric);
        let mut run = DistanceRun::new(&samples, metric);
        assert_eq!(run.rows(), 40);
        let stepped = stepped(3, &mut |pace| run.advance(pace));
        assert_eq!(stepped.result, whole, "{metric:?}");
        assert!(stepped.pauses >= 10);
        assert_monotone(&stepped.reports);
        // Every row is announced before it is computed.
        assert_eq!(stepped.reports.last(), Some(&(39, 40)));
    }
}

#[test]
fn nmds_is_the_same_wherever_it_pauses() {
    let (samples, _) = table(30, 20);
    let matrix = distances(&samples, Distance::BrayCurtis);
    let whole = nmds(&matrix, 2).unwrap();
    let mut run = NmdsRun::new(2);
    let stepped = stepped(2, &mut |pace| run.advance(&matrix, pace));
    assert_eq!(stepped.result.unwrap(), whole);
    assert!(stepped.pauses >= 5);
    assert_monotone(&stepped.reports);
    // It says when it has finished, however few iterations it took.
    assert_eq!(
        stepped.reports.last(),
        Some(&(NmdsRun::iterations(), NmdsRun::iterations()))
    );
    // Too few samples: no layout, as from the function.
    let small = distances(&samples[..2], Distance::BrayCurtis);
    let mut run = NmdsRun::new(2);
    assert_eq!(
        run.advance(&small, &mut |_, _| Control::Continue),
        Step::Done(None)
    );
}

#[test]
fn community_tests_are_the_same_wherever_they_pause() {
    let (samples, groups) = table(36, 20);
    let matrix = distances(&samples, Distance::BrayCurtis);
    let ordination = pcoa(&matrix).unwrap();
    for test in CommunityTest::ALL {
        let whole = test.run_with(
            &matrix,
            &groups,
            199,
            &mut rng_from_seed(5),
            Some(&ordination),
        );
        let mut run = test.start(&matrix, &groups, 199, Some(&ordination));
        let mut rng = rng_from_seed(5);
        let stepped = stepped(7, &mut |pace| run.advance(&mut rng, pace));
        assert_eq!(stepped.result, whole, "{test:?}");
        assert!(stepped.pauses >= 20);
        assert_monotone(&stepped.reports);
        assert_eq!(stepped.reports.len(), 199 + stepped.pauses);
        // The random numbers left over are the same too, so that tests run
        // one after another give the same results.
        let mut after_whole = rng_from_seed(5);
        test.run_with(&matrix, &groups, 199, &mut after_whole, Some(&ordination));
        assert_eq!(
            rand::Rng::next_u64(&mut rng),
            rand::Rng::next_u64(&mut after_whole)
        );
    }
}

#[test]
fn a_cancelled_run_stops() {
    let (samples, groups) = table(20, 10);
    let matrix = distances(&samples, Distance::BrayCurtis);
    let mut run = CommunityTest::Permanova.start(&matrix, &groups, 99, None);
    let mut calls = 0;
    let step = run.advance(&mut rng_from_seed(1), &mut |_, _| {
        calls += 1;
        if calls > 10 {
            Control::Cancel
        } else {
            Control::Continue
        }
    });
    assert_eq!(step, Step::Cancelled);
    assert_eq!(calls, 11);
}

#[test]
fn the_mantel_test_is_the_same_wherever_it_pauses() {
    let (samples, _) = table(25, 15);
    let matrix = distances(&samples, Distance::BrayCurtis);
    let values: Vec<f64> = (0..25).map(|i| ((i * 37) % 11) as f64).collect();
    let n = values.len();
    let other = distances(
        &values.iter().map(|v| vec![*v, 1.0]).collect::<Vec<_>>(),
        Distance::Aitchison,
    );
    assert_eq!(other.size(), n);
    for method in [CorrelationMethod::Pearson, CorrelationMethod::Spearman] {
        let whole = mantel(&other, &matrix, method, 99, &mut rng_from_seed(2));
        let mut run = MantelRun::new(&other, &matrix, method, 99);
        let mut rng = rng_from_seed(2);
        let stepped = stepped(4, &mut |pace| run.advance(&mut rng, pace));
        assert_eq!(stepped.result, whole, "{method:?}");
        assert!(stepped.pauses >= 20);
        assert_monotone(&stepped.reports);
    }
}

#[test]
fn ancom_is_the_same_wherever_it_pauses() {
    let (samples, groups) = table(24, 30);
    for test in [MultiGroupTest::Anova, MultiGroupTest::KruskalWallis] {
        let whole = ancom(&samples, &groups, test, 0.05).unwrap();
        let mut run = AncomRun::new(&samples, &groups, test, 0.05).unwrap();
        assert_eq!(run.features(), 30);
        let stepped = stepped(3, &mut |pace| run.advance(pace));
        assert_eq!(stepped.result, whole, "{test:?}");
        assert!(stepped.pauses >= 9);
        assert_monotone(&stepped.reports);
    }
}
