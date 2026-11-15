//! Starting Claude Code connected to STAMP, in the computer's own terminal.
//!
//! STAMP prepares a working folder in its data folder, rewritten at each
//! launch:
//!
//! - `stamp-mcp.json`: where STAMP's server is and its access token for
//!   this run, given to Claude Code with `--mcp-config`;
//! - `stamp-settings.json`: allows STAMP's own tools without asking each
//!   time (anything else Claude Code does still asks first), given with
//!   `--settings` because a project's own `.claude/settings.json` is only
//!   read once the user has trusted the folder;
//! - `CLAUDE.md`: what STAMP is and how to help its users;
//! - a script that starts Claude Code there, which a terminal runs.
//!
//! Claude Code itself is the user's own installation, signed in with their
//! own Claude account; STAMP never sees the sign-in. If it is not
//! installed, STAMP can open a terminal that runs Anthropic's installer
//! and then starts it.

use std::path::{Path, PathBuf};

/// Anthropic's installers.
const INSTALL_SH: &str = "https://claude.ai/install.sh";
const INSTALL_PS1: &str = "https://claude.ai/install.ps1";

/// Name of the MCP configuration file in the working folder.
const MCP_CONFIG: &str = "stamp-mcp.json";
/// Name of the settings file in the working folder.
const SETTINGS: &str = "stamp-settings.json";
/// The arguments that connect Claude Code to STAMP.
fn claude_arguments() -> String {
    format!("--mcp-config {MCP_CONFIG} --settings {SETTINGS}")
}

/// The program and arguments that run Claude Code (`claude`), connected to
/// STAMP, in the prepared folder: for the Claude Code panel.
pub fn claude_command(claude: &Path) -> (String, Vec<String>) {
    let mut args = vec![
        "--mcp-config".to_string(),
        MCP_CONFIG.to_string(),
        "--settings".to_string(),
        SETTINGS.to_string(),
    ];
    let program = claude.display().to_string();
    // npm installs a batch file on Windows, which only cmd can run.
    let is_batch = claude
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"));
    if cfg!(windows) && is_batch {
        let mut cmd = vec!["/c".to_string(), program];
        cmd.append(&mut args);
        return ("cmd".to_string(), cmd);
    }
    (program, args)
}

/// The program and arguments that run the install script written by
/// [`prepare_install`], in the Claude Code panel.
pub fn script_command(script: &Path) -> (String, Vec<String>) {
    let script = script.display().to_string();
    if cfg!(windows) {
        let args = ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]
            .iter()
            .map(|a| a.to_string())
            .chain([script])
            .collect();
        ("powershell".to_string(), args)
    } else {
        ("/bin/sh".to_string(), vec![script])
    }
}

/// The folder Claude Code runs in.
pub fn working_folder() -> Option<PathBuf> {
    eframe::storage_dir("STAMP").map(|dir| dir.join("assistant"))
}

/// The user's home folder.
fn home() -> Option<PathBuf> {
    std::env::home_dir()
}

fn executable_name() -> &'static str {
    if cfg!(windows) {
        "claude.exe"
    } else {
        "claude"
    }
}

/// Look for an executable on the `PATH`.
fn on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Where Claude Code is installed, if it is. Programs started from the
/// desktop (Finder, the Start menu) often have a short `PATH`, so the
/// places its installer and npm use are looked in too.
pub fn find_claude() -> Option<PathBuf> {
    let name = executable_name();
    if let Some(found) = on_path(name) {
        return Some(found);
    }
    let home = home()?;
    let mut places = vec![
        home.join(".local").join("bin"),
        home.join(".claude").join("local"),
        home.join(".npm-global").join("bin"),
    ];
    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            places.push(PathBuf::from(appdata).join("npm"));
        }
    } else {
        places.push(PathBuf::from("/usr/local/bin"));
        places.push(PathBuf::from("/opt/homebrew/bin"));
    }
    places
        .into_iter()
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .or_else(|| {
            // npm on Windows installs a .cmd wrapper.
            cfg!(windows)
                .then(|| std::env::var_os("APPDATA"))
                .flatten()
                .map(|appdata| PathBuf::from(appdata).join("npm").join("claude.cmd"))
                .filter(|candidate| candidate.is_file())
        })
}

/// What Claude Code is told about STAMP, in `CLAUDE.md`.
const GUIDE: &str = "\
# Helping a STAMP user

You were started from STAMP (statistical analysis of taxonomic and functional
profiles), which is open beside this terminal. STAMP's MCP server (`stamp`)
gives you its tools: they show you the user's data, the results of the
current analysis and the plot on screen, and let you change the analysis and
the plot. Changes appear in STAMP's window at once and the user can undo them
there (or ask you to, with the `undo` tool).

- Start with `get_session` and `get_current_plot` to see what the user sees.
- The user is most likely a biologist rather than a statistician or a
  programmer. Explain in plain language, and say what the statistics do not
  show as well as what they do: effect sizes and their confidence intervals
  matter more than p-values; corrected p-values account for the many features
  tested; a small number of samples limits what can be concluded.
- Prefer STAMP's own plots (`list_plots`, `show_plot`) to drawing your own.
- Do not change files on the user's computer unless they ask you to.
";

/// Write the working folder for a server at `url` with access `token`, and
/// the script that starts Claude Code (`claude`) in it. Returns the
/// script.
pub fn prepare(folder: &Path, url: &str, token: &str, claude: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(folder)?;
    let config = serde_json::json!({
        "mcpServers": {
            "stamp": {
                "type": "http",
                "url": url,
                "headers": {"Authorization": format!("Bearer {token}")}
            }
        }
    });
    write_private(
        &folder.join(MCP_CONFIG),
        &serde_json::to_string_pretty(&config).unwrap_or_default(),
    )?;
    let settings = serde_json::json!({
        "permissions": {"allow": ["mcp__stamp"]}
    });
    std::fs::write(
        folder.join(SETTINGS),
        serde_json::to_string_pretty(&settings).unwrap_or_default(),
    )?;
    std::fs::write(folder.join("CLAUDE.md"), GUIDE)?;
    write_script(folder, "start-claude", &start_commands(claude))
}

/// The commands of the script that starts Claude Code.
fn start_commands(claude: &Path) -> String {
    let claude = claude.display();
    if cfg!(windows) {
        format!("\"{claude}\" {}\r\n", claude_arguments())
    } else {
        format!(
            "exec '{}' {}\n",
            shell_quoted(&claude.to_string()),
            claude_arguments()
        )
    }
}

/// The text inside single quotes for a POSIX shell.
fn shell_quoted(text: &str) -> String {
    text.replace('\'', "'\\''")
}

/// Write the folder's files and a script that installs Claude Code with
/// Anthropic's installer and then starts it. Returns the script.
pub fn prepare_install(folder: &Path, url: &str, token: &str) -> std::io::Result<PathBuf> {
    let home = home().unwrap_or_default();
    let installed = home.join(".local").join("bin").join(executable_name());
    prepare(folder, url, token, &installed)?;
    if cfg!(windows) {
        // A PowerShell script: the installer is a PowerShell one.
        let script = folder.join("install-claude.ps1");
        let text = format!(
            "Set-Location -LiteralPath $PSScriptRoot\r\n\
             Write-Host 'Installing Claude Code from {INSTALL_PS1} ...'\r\n\
             irm {INSTALL_PS1} | iex\r\n\
             $claude = Join-Path $env:USERPROFILE '.local\\bin\\claude.exe'\r\n\
             if (Test-Path $claude) {{ & $claude {args} }}\r\n\
             else {{ Write-Host 'Claude Code was not installed where expected.' }}\r\n",
            args = claude_arguments()
        );
        std::fs::write(&script, text)?;
        Ok(script)
    } else {
        let commands = format!(
            "echo 'Installing Claude Code from {INSTALL_SH} ...'\n\
             curl -fsSL {INSTALL_SH} | bash\n\
             if [ -x '{0}' ]; then exec '{0}' {1}; fi\n\
             echo 'Claude Code was not installed where expected.'\n\
             exec \"${{SHELL:-/bin/sh}}\"\n",
            shell_quoted(&installed.display().to_string()),
            claude_arguments()
        );
        write_script(folder, "install-claude", &commands)
    }
}

/// A file only its owner can read: it holds the access token.
pub(crate) fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    std::fs::write(path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// A script that runs `commands` in `folder`: `name.cmd` on Windows,
/// `name.command` on macOS (which Terminal opens), `name.sh` elsewhere.
fn write_script(folder: &Path, name: &str, commands: &str) -> std::io::Result<PathBuf> {
    if cfg!(windows) {
        let path = folder.join(format!("{name}.cmd"));
        std::fs::write(&path, format!("@echo off\r\ncd /d \"%~dp0\"\r\n{commands}"))?;
        return Ok(path);
    }
    let extension = if cfg!(target_os = "macos") {
        "command"
    } else {
        "sh"
    };
    let path = folder.join(format!("{name}.{extension}"));
    std::fs::write(
        &path,
        format!("#!/bin/sh\ncd \"$(dirname \"$0\")\" || exit 1\n{commands}"),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(path)
}

/// Terminal programs of Linux and other Unix desktops, with the arguments
/// that come before the command they run, in the order they are tried.
const UNIX_TERMINALS: [(&str, &[&str]); 10] = [
    ("x-terminal-emulator", &["-e"]),
    ("gnome-terminal", &["--"]),
    ("konsole", &["-e"]),
    ("xfce4-terminal", &["-x"]),
    ("mate-terminal", &["-x"]),
    ("tilix", &["-e"]),
    ("kitty", &[]),
    ("alacritty", &["-e"]),
    ("wezterm", &["start", "--"]),
    ("xterm", &["-e"]),
];

/// The program and arguments that open a terminal running `script`, on
/// Linux and other Unix desktops. `exists` says whether a program is
/// installed. `$TERMINAL`, if set, is tried first.
fn unix_terminal(
    script: &Path,
    preferred: Option<String>,
    exists: impl Fn(&str) -> bool,
) -> Option<(String, Vec<String>)> {
    let script = script.display().to_string();
    if let Some(preferred) = preferred.filter(|t| exists(t)) {
        return Some((preferred, vec!["-e".to_string(), script]));
    }
    UNIX_TERMINALS
        .iter()
        .find(|(program, _)| exists(program))
        .map(|(program, before)| {
            let mut args: Vec<String> = before.iter().map(|a| a.to_string()).collect();
            args.push(script);
            (program.to_string(), args)
        })
}

/// Open the computer's terminal running `script`.
pub fn open_terminal(script: &Path) -> Result<(), String> {
    let folder = script.parent().unwrap_or(Path::new("."));
    let mut command = if cfg!(windows) {
        let is_powershell = script.extension().is_some_and(|e| e == "ps1");
        let run: Vec<String> = if is_powershell {
            vec![
                "powershell".into(),
                "-NoExit".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                script.display().to_string(),
            ]
        } else {
            vec!["cmd".into(), "/k".into(), script.display().to_string()]
        };
        if on_path("wt.exe").is_some() {
            let mut command = std::process::Command::new("wt.exe");
            command.arg("-d").arg(folder).args(run);
            command
        } else {
            let mut command = std::process::Command::new("cmd");
            command.args(["/c", "start", "Claude Code"]).args(run);
            command
        }
    } else if cfg!(target_os = "macos") {
        let mut command = std::process::Command::new("open");
        command.args(["-a", "Terminal"]).arg(script);
        command
    } else {
        let (program, args) = unix_terminal(script, std::env::var("TERMINAL").ok(), |name| {
            on_path(name).is_some()
        })
        .ok_or(
            "No terminal program was found. Open a terminal and run the script in STAMP's \
             assistant folder yourself.",
        )?;
        let mut command = std::process::Command::new(program);
        command.args(args);
        command
    };
    command
        .current_dir(folder)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open a terminal: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("stamp-assistant-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn prepares_the_working_folder() {
        let folder = scratch("prepare");
        let script = prepare(
            &folder,
            "http://127.0.0.1:4321/mcp",
            "secret",
            Path::new("/opt/it's/claude"),
        )
        .unwrap();
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join(MCP_CONFIG)).unwrap())
                .unwrap();
        let stamp = &config["mcpServers"]["stamp"];
        assert_eq!(stamp["type"], "http");
        assert_eq!(stamp["url"], "http://127.0.0.1:4321/mcp");
        assert_eq!(stamp["headers"]["Authorization"], "Bearer secret");
        let settings = std::fs::read_to_string(folder.join(SETTINGS)).unwrap();
        assert!(settings.contains("mcp__stamp"));
        assert!(folder.join("CLAUDE.md").is_file());
        let text = std::fs::read_to_string(&script).unwrap();
        assert!(
            text.contains("--mcp-config stamp-mcp.json --settings stamp-settings.json"),
            "{text}"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert!(text.contains("'/opt/it'\\''s/claude'"), "{text}");
            let mode = std::fs::metadata(folder.join(MCP_CONFIG))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "the token must be private");
        }
        // Prepared again for another run: the files are replaced.
        prepare(
            &folder,
            "http://127.0.0.1:9/mcp",
            "other",
            Path::new("claude"),
        )
        .unwrap();
        let config = std::fs::read_to_string(folder.join(MCP_CONFIG)).unwrap();
        assert!(config.contains("Bearer other") && !config.contains("secret"));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn install_script_runs_anthropics_installer() {
        let folder = scratch("install");
        let script = prepare_install(&folder, "http://127.0.0.1:1/mcp", "t").unwrap();
        let text = std::fs::read_to_string(&script).unwrap();
        assert!(text.contains(if cfg!(windows) {
            INSTALL_PS1
        } else {
            INSTALL_SH
        }));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn commands_for_the_panel() {
        let (program, args) = claude_command(Path::new("/x/claude"));
        assert_eq!(program, "/x/claude");
        assert_eq!(
            args,
            ["--mcp-config", MCP_CONFIG, "--settings", SETTINGS].map(String::from)
        );
        let (program, args) = script_command(Path::new("/x/install-claude.sh"));
        if cfg!(windows) {
            assert_eq!(program, "powershell");
        } else {
            assert_eq!(
                (program.as_str(), args[0].as_str()),
                ("/bin/sh", "/x/install-claude.sh")
            );
        }
    }

    #[test]
    fn chooses_an_installed_terminal() {
        let script = Path::new("/x/start-claude.sh");
        let only = |wanted: &'static str| move |name: &str| name == wanted;
        assert_eq!(
            unix_terminal(script, None, only("gnome-terminal")),
            Some((
                "gnome-terminal".to_string(),
                vec!["--".to_string(), "/x/start-claude.sh".to_string()]
            ))
        );
        assert_eq!(
            unix_terminal(script, None, only("kitty")).unwrap().1,
            ["/x/start-claude.sh"]
        );
        assert_eq!(
            unix_terminal(script, Some("foot".to_string()), |_| true)
                .unwrap()
                .0,
            "foot"
        );
        assert_eq!(unix_terminal(script, None, |_| false), None);
    }
}
