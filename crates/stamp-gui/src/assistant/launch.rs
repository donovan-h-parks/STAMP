//! Starting Claude Code, OpenCode or Google's Antigravity CLI connected to
//! STAMP, in the assistant panel or the computer's own terminal.
//!
//! STAMP prepares a working folder in its data folder, rewritten at each
//! launch. For Claude Code:
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
//! For OpenCode, `opencode.json` (the server, its token, and rules that
//! make OpenCode ask before running commands, editing files or fetching
//! web pages), which OpenCode reads as the configuration of the folder it
//! starts in, and `AGENTS.md`, its counterpart of `CLAUDE.md`.
//!
//! For Antigravity (`agy`), `.agents/mcp_config.json` (the server and its
//! token), which it reads as the MCP servers of the folder (its
//! workspace), `stamp-antigravity-settings.json`, given with `--settings`
//! (settings for that run only, never saved), which allows STAMP's own
//! tools without asking, and `AGENTS.md`.
//!
//! Each program is the user's own installation, signed in with their own
//! account; STAMP never sees the sign-in. If it is not installed, STAMP can
//! run its maker's installer and then start it.

use std::path::{Path, PathBuf};

/// Anthropic's installers.
const INSTALL_SH: &str = "https://claude.ai/install.sh";
const INSTALL_PS1: &str = "https://claude.ai/install.ps1";
/// OpenCode's installer (OpenCode 2) for Linux and macOS. For Windows,
/// which it has no installer for, the version is looked up here and the
/// program taken from its GitHub release.
const OPENCODE_INSTALL_SH: &str = "https://opencode.ai/v2/install";
const OPENCODE_LATEST: &str = "https://opencode.ai/update/api/latest/cli/npm";
const OPENCODE_RELEASES: &str = "https://github.com/anomalyco/opencode/releases/download";
/// OpenCode's configuration file in the working folder.
const OPENCODE_CONFIG: &str = "opencode.json";
/// How long, in milliseconds, OpenCode waits for one of STAMP's tools:
/// as long as STAMP's server waits for the window (a slow plot can take a
/// while).
const OPENCODE_TOOL_TIMEOUT: u64 = 15 * 60 * 1000;
/// Google's installers for the Antigravity CLI.
const ANTIGRAVITY_INSTALL_SH: &str = "https://antigravity.google/cli/install.sh";
const ANTIGRAVITY_INSTALL_PS1: &str = "https://antigravity.google/cli/install.ps1";
/// Antigravity's MCP configuration in the working folder (its workspace).
const ANTIGRAVITY_CONFIG: [&str; 2] = [".agents", "mcp_config.json"];
/// Antigravity's settings for the run, in the working folder.
const ANTIGRAVITY_SETTINGS: &str = "stamp-antigravity-settings.json";

/// The programs that can run in the assistant panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Agent {
    /// Anthropic's Claude Code, with a Claude subscription.
    #[default]
    ClaudeCode,
    /// OpenCode (opencode.ai), with a ChatGPT or GitHub Copilot
    /// subscription, among others.
    OpenCode,
    /// Google's Antigravity CLI (`agy`), with a Google account.
    Antigravity,
}

impl Agent {
    pub const ALL: [Agent; 3] = [Agent::OpenCode, Agent::ClaudeCode, Agent::Antigravity];

    /// Its name, as shown in menus and on the panel.
    pub fn name(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "Claude Code",
            Agent::OpenCode => "OpenCode",
            Agent::Antigravity => "Google Antigravity",
        }
    }

    /// The accounts it can be used with.
    pub fn accounts(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "a Claude Pro, Max, Team or Enterprise plan",
            Agent::OpenCode => {
                "a ChatGPT Plus or Pro plan, GitHub Copilot, OpenCode's own plans, or an API \
                 key from many providers; it also has a few free models"
            }
            Agent::Antigravity => {
                "a Google account: free with a weekly limit, or more with a Google AI Pro or \
                 Ultra plan"
            }
        }
    }

    /// Where it is installed, if it is.
    pub fn find(self) -> Option<PathBuf> {
        match self {
            Agent::ClaudeCode => find_claude(),
            Agent::OpenCode => find_opencode(),
            Agent::Antigravity => find_antigravity(),
        }
    }

    /// The program and arguments that run it, installed at `program`,
    /// connected to STAMP, in the prepared folder.
    pub fn command(self, program: &Path) -> (String, Vec<String>) {
        match self {
            Agent::ClaudeCode => claude_command(program),
            Agent::OpenCode => opencode_command(program),
            Agent::Antigravity => wrapped(program, antigravity_arguments()),
        }
    }

    /// Write the working folder for a server at `url` with access `token`,
    /// and a script that starts the program (installed at `program`) in it.
    /// Returns the script.
    pub fn prepare(
        self,
        folder: &Path,
        url: &str,
        token: &str,
        program: &Path,
    ) -> std::io::Result<PathBuf> {
        match self {
            Agent::ClaudeCode => prepare(folder, url, token, program),
            Agent::OpenCode => prepare_opencode(folder, url, token, program),
            Agent::Antigravity => prepare_antigravity(folder, url, token, program),
        }
    }

    /// Write the working folder and a script that installs the program
    /// with its maker's installer and then starts it. Returns the script.
    pub fn prepare_install(
        self,
        folder: &Path,
        url: &str,
        token: &str,
    ) -> std::io::Result<PathBuf> {
        match self {
            Agent::ClaudeCode => prepare_install(folder, url, token),
            Agent::OpenCode => prepare_opencode_install(folder, url, token),
            Agent::Antigravity => prepare_antigravity_install(folder, url, token),
        }
    }

    /// Where its installer is, as told to the user.
    pub fn installer(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "Anthropic's installer (claude.ai/install)",
            Agent::OpenCode if cfg!(windows) => "OpenCode's release on GitHub",
            Agent::OpenCode => "OpenCode's installer (opencode.ai/v2/install)",
            Agent::Antigravity => "Google's installer (antigravity.google/cli/install)",
        }
    }
}

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
    let args = vec![
        "--mcp-config".to_string(),
        MCP_CONFIG.to_string(),
        "--settings".to_string(),
        SETTINGS.to_string(),
    ];
    wrapped(claude, args)
}

/// `program` with `args`, run through cmd if it is a batch file (which
/// npm installs on Windows, and only cmd can run).
fn wrapped(program: &Path, mut args: Vec<String>) -> (String, Vec<String>) {
    let is_batch = program
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"));
    let program = program.display().to_string();
    if cfg!(windows) && is_batch {
        let mut cmd = vec!["/c".to_string(), program];
        cmd.append(&mut args);
        return ("cmd".to_string(), cmd);
    }
    (program, args)
}

/// The arguments that start OpenCode, of major version `major` if known.
/// OpenCode 2 normally runs one background server shared by all its
/// windows, which would keep the connection to STAMP of an earlier run;
/// `--standalone` gives it a server of its own that reads the folder's
/// configuration afresh. OpenCode 1 always did that and has no such
/// option.
fn opencode_arguments(major: Option<u32>) -> Vec<String> {
    if major == Some(1) {
        Vec::new()
    } else {
        vec!["--standalone".to_string()]
    }
}

/// The program and arguments that run OpenCode, installed at `opencode`,
/// connected to STAMP, in the prepared folder.
pub fn opencode_command(opencode: &Path) -> (String, Vec<String>) {
    wrapped(opencode, opencode_arguments(opencode_major(opencode)))
}

/// The major version of OpenCode installed at `opencode`, from
/// `opencode --version` ("opencode v2.3.0", or "1.19.5" from OpenCode 1).
/// Remembered for each installation, so that preparing the folder and
/// starting it ask once.
pub fn opencode_major(opencode: &Path) -> Option<u32> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type Known = HashMap<(PathBuf, Option<std::time::SystemTime>), Option<u32>>;
    static KNOWN: Mutex<Option<Known>> = Mutex::new(None);
    let modified = std::fs::metadata(opencode).and_then(|m| m.modified()).ok();
    let key = (opencode.to_path_buf(), modified);
    if let Some(&major) = KNOWN.lock().ok()?.get_or_insert_default().get(&key) {
        return major;
    }
    let major = ask_version(opencode).as_deref().and_then(major_version);
    if let Ok(mut known) = KNOWN.lock() {
        known.get_or_insert_default().insert(key, major);
    }
    major
}

/// What `program --version` prints, if it answers within a few seconds.
fn ask_version(program: &Path) -> Option<String> {
    use std::io::Read;
    let (name, args) = wrapped(program, vec!["--version".to_string()]);
    let mut command = std::process::Command::new(name);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        /// No console window flashing up.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().ok()?;
    let end = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < end => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut text = String::new();
    child.stdout.take()?.read_to_string(&mut text).ok()?;
    Some(text)
}

/// The major version in what a program printed for `--version`: its last
/// word, without a leading "v".
fn major_version(text: &str) -> Option<u32> {
    let word = text.split_whitespace().last()?;
    word.trim_start_matches('v').split('.').next()?.parse().ok()
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

fn opencode_name() -> &'static str {
    if cfg!(windows) {
        "opencode.exe"
    } else {
        "opencode"
    }
}

/// Where OpenCode's installers put it.
fn opencode_installed_at() -> PathBuf {
    home()
        .unwrap_or_default()
        .join(".opencode")
        .join("bin")
        .join(opencode_name())
}

fn antigravity_name() -> &'static str {
    if cfg!(windows) {
        "agy.exe"
    } else {
        "agy"
    }
}

/// Where Google's installers put the Antigravity CLI: `~/.local/bin`, or
/// `%LOCALAPPDATA%\agy\bin` on Windows.
fn antigravity_installed_at() -> PathBuf {
    let folder = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().unwrap_or_default().join("AppData").join("Local"))
            .join("agy")
            .join("bin")
    } else {
        home().unwrap_or_default().join(".local").join("bin")
    };
    folder.join(antigravity_name())
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
    find_program(
        executable_name(),
        "claude",
        &[&[".local", "bin"], &[".claude", "local"]],
    )
}

/// Where OpenCode is installed, if it is: on the `PATH`, or where its
/// installers, npm, bun or Homebrew put it.
pub fn find_opencode() -> Option<PathBuf> {
    find_program(
        opencode_name(),
        "opencode",
        &[&[".opencode", "bin"], &[".local", "bin"], &[".bun", "bin"]],
    )
}

/// Where the Antigravity CLI is installed, if it is: on the `PATH`, or
/// where Google's installers put it.
pub fn find_antigravity() -> Option<PathBuf> {
    on_path(antigravity_name())
        .or_else(|| Some(antigravity_installed_at()).filter(|path| path.is_file()))
}

/// The executable `name` on the `PATH`, in one of the folders `in_home`
/// (each the parts of a path under the home folder), or where npm and
/// Homebrew install programs; on Windows also `stem.cmd`, npm's wrapper.
fn find_program(name: &str, stem: &str, in_home: &[&[&str]]) -> Option<PathBuf> {
    if let Some(found) = on_path(name) {
        return Some(found);
    }
    let home = home()?;
    let mut places: Vec<PathBuf> = in_home
        .iter()
        .map(|parts| {
            parts
                .iter()
                .fold(home.clone(), |path, part| path.join(part))
        })
        .collect();
    places.push(home.join(".npm-global").join("bin"));
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
                .map(|appdata| {
                    PathBuf::from(appdata)
                        .join("npm")
                        .join(format!("{stem}.cmd"))
                })
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
- Reach STAMP only through these tools. If you do not see them, say so and
  stop: do not read the files of this folder or write a program to connect
  to STAMP yourself.
- The user is most likely a biologist rather than a statistician or a
  programmer. Explain in plain language, and say what the statistics do not
  show as well as what they do: effect sizes and their confidence intervals
  matter more than p-values; corrected p-values account for the many features
  tested; a small number of samples limits what can be concluded.
- Prefer STAMP's own plots (`list_plots`, `show_plot`). When none shows what
  the user asks for, draw one with `show_custom_plot`: describe it in
  Vega-Lite, using STAMP's data by name (`profile`, `samples`, `results`), and
  check it with `get_current_plot`.
- When the user asks for other data, open it with `load_data`, giving full
  paths. It replaces the data loaded now, so use it only when asked.
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

/// OpenCode's configuration for the working folder: STAMP's server, with
/// OpenCode's own sign-in for servers (OAuth) off since STAMP gives a
/// token, and rules that make it ask before it runs commands, edits files
/// or fetches web pages. OpenCode 2 lists STAMP's tools to the model one
/// by one (`codemode: false`) rather than through code it writes, which
/// keeps the plot images they return; OpenCode 1 takes the older form,
/// which OpenCode 2 also reads but without those settings.
fn opencode_config(url: &str, token: &str, major: Option<u32>) -> serde_json::Value {
    let authorization = format!("Bearer {token}");
    if major == Some(1) {
        return serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "mcp": {
                "stamp": {
                    "type": "remote",
                    "url": url,
                    "headers": {"Authorization": authorization},
                    "oauth": false,
                    "enabled": true,
                    "timeout": OPENCODE_TOOL_TIMEOUT
                }
            },
            "permission": {"bash": "ask", "edit": "ask", "webfetch": "ask"}
        });
    }
    let rule = |action: &str, effect: &str| serde_json::json!({"action": action, "resource": "*", "effect": effect});
    serde_json::json!({
        "$schema": "https://opencode.ai/config.json",
        "mcp": {
            "servers": {
                "stamp": {
                    "type": "remote",
                    "url": url,
                    "headers": {"Authorization": authorization},
                    "oauth": false,
                    "codemode": false,
                    "timeout": {"execution": OPENCODE_TOOL_TIMEOUT}
                }
            }
        },
        // The last rule that matches decides.
        "permissions": [
            rule("shell", "ask"),
            rule("edit", "ask"),
            rule("webfetch", "ask"),
            rule("stamp_*", "allow"),
        ]
    })
}

/// Write the working folder for OpenCode (installed at `opencode`) and a
/// script that starts it there. Returns the script.
pub fn prepare_opencode(
    folder: &Path,
    url: &str,
    token: &str,
    opencode: &Path,
) -> std::io::Result<PathBuf> {
    let major = opencode
        .is_file()
        .then(|| opencode_major(opencode))
        .flatten();
    write_opencode_folder(folder, url, token, major)?;
    let arguments = opencode_arguments(major).join(" ");
    let commands = if cfg!(windows) {
        format!("\"{}\" {arguments}\r\n", opencode.display())
    } else {
        format!(
            "exec '{}' {arguments}\n",
            shell_quoted(&opencode.display().to_string())
        )
    };
    write_script(folder, "start-opencode", &commands)
}

/// The configuration and instructions of OpenCode's working folder.
fn write_opencode_folder(
    folder: &Path,
    url: &str,
    token: &str,
    major: Option<u32>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(folder)?;
    write_private(
        &folder.join(OPENCODE_CONFIG),
        &serde_json::to_string_pretty(&opencode_config(url, token, major)).unwrap_or_default(),
    )?;
    std::fs::write(folder.join("AGENTS.md"), GUIDE)
}

/// Write OpenCode's working folder and a script that installs OpenCode 2
/// and then starts it. Returns the script.
pub fn prepare_opencode_install(folder: &Path, url: &str, token: &str) -> std::io::Result<PathBuf> {
    write_opencode_folder(folder, url, token, None)?;
    let installed = opencode_installed_at();
    let arguments = opencode_arguments(None).join(" ");
    if cfg!(windows) {
        let script = folder.join("install-opencode.ps1");
        let text = format!(
            "Set-Location -LiteralPath $PSScriptRoot\r\n\
             $ErrorActionPreference = 'Stop'\r\n\
             $ProgressPreference = 'SilentlyContinue'\r\n\
             [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12\r\n\
             Write-Host 'Installing OpenCode from {OPENCODE_RELEASES} ...'\r\n\
             $version = (Invoke-RestMethod '{OPENCODE_LATEST}').version\r\n\
             $arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {{ 'arm64' }} else {{ 'x64-baseline' }}\r\n\
             $zip = Join-Path $env:TEMP \"opencode-$version.zip\"\r\n\
             Invoke-WebRequest -UseBasicParsing \"{OPENCODE_RELEASES}/v$version/opencode-windows-$arch.zip\" -OutFile $zip\r\n\
             $dir = Split-Path -Parent '{installed}'\r\n\
             New-Item -ItemType Directory -Force -Path $dir | Out-Null\r\n\
             Expand-Archive -Force -Path $zip -DestinationPath $dir\r\n\
             Remove-Item $zip\r\n\
             Write-Host \"OpenCode $version is installed in $dir.\"\r\n\
             if (Test-Path '{installed}') {{ & '{installed}' {arguments} }}\r\n\
             else {{ Write-Host 'OpenCode was not installed where expected.' }}\r\n",
            installed = installed.display().to_string().replace('\'', "''"),
        );
        std::fs::write(&script, text)?;
        Ok(script)
    } else {
        let commands = format!(
            "echo 'Installing OpenCode from {OPENCODE_INSTALL_SH} ...'\n\
             curl -fsSL {OPENCODE_INSTALL_SH} | bash\n\
             if [ -x '{0}' ]; then exec '{0}' {arguments}; fi\n\
             echo 'OpenCode was not installed where expected.'\n\
             exec \"${{SHELL:-/bin/sh}}\"\n",
            shell_quoted(&installed.display().to_string()),
        );
        write_script(folder, "install-opencode", &commands)
    }
}

/// The arguments that start Antigravity with its settings for the run.
fn antigravity_arguments() -> Vec<String> {
    vec!["--settings".to_string(), ANTIGRAVITY_SETTINGS.to_string()]
}

/// Antigravity's MCP configuration for the working folder: STAMP's server
/// and its token, with as long to answer as STAMP's server waits for the
/// window.
fn antigravity_config(url: &str, token: &str) -> serde_json::Value {
    serde_json::json!({
        "mcpServers": {
            "stamp": {
                "url": url,
                "headers": {"Authorization": format!("Bearer {token}")},
                "timeoutSeconds": OPENCODE_TOOL_TIMEOUT / 1000
            }
        }
    })
}

/// The configuration and instructions of Antigravity's working folder.
fn write_antigravity_folder(folder: &Path, url: &str, token: &str) -> std::io::Result<()> {
    let [dir, file] = ANTIGRAVITY_CONFIG;
    std::fs::create_dir_all(folder.join(dir))?;
    write_private(
        &folder.join(dir).join(file),
        &serde_json::to_string_pretty(&antigravity_config(url, token)).unwrap_or_default(),
    )?;
    // Ask rules and the user's own rules still apply to everything else.
    let settings = serde_json::json!({"permissions": {"allow": ["mcp(stamp/*)"]}});
    std::fs::write(
        folder.join(ANTIGRAVITY_SETTINGS),
        serde_json::to_string_pretty(&settings).unwrap_or_default(),
    )?;
    std::fs::write(folder.join("AGENTS.md"), GUIDE)
}

/// Write the working folder for the Antigravity CLI (installed at `agy`)
/// and a script that starts it there. Returns the script.
pub fn prepare_antigravity(
    folder: &Path,
    url: &str,
    token: &str,
    agy: &Path,
) -> std::io::Result<PathBuf> {
    write_antigravity_folder(folder, url, token)?;
    let arguments = antigravity_arguments().join(" ");
    let commands = if cfg!(windows) {
        format!("\"{}\" {arguments}\r\n", agy.display())
    } else {
        format!(
            "exec '{}' {arguments}\n",
            shell_quoted(&agy.display().to_string())
        )
    };
    write_script(folder, "start-antigravity", &commands)
}

/// Write Antigravity's working folder and a script that installs the
/// Antigravity CLI with Google's installer and then starts it. Returns the
/// script.
pub fn prepare_antigravity_install(
    folder: &Path,
    url: &str,
    token: &str,
) -> std::io::Result<PathBuf> {
    write_antigravity_folder(folder, url, token)?;
    let installed = antigravity_installed_at();
    let arguments = antigravity_arguments().join(" ");
    if cfg!(windows) {
        let script = folder.join("install-antigravity.ps1");
        let text = format!(
            "Set-Location -LiteralPath $PSScriptRoot\r\n\
             Write-Host 'Installing the Antigravity CLI from {ANTIGRAVITY_INSTALL_PS1} ...'\r\n\
             irm {ANTIGRAVITY_INSTALL_PS1} | iex\r\n\
             if (Test-Path '{installed}') {{ & '{installed}' {arguments} }}\r\n\
             else {{ Write-Host 'The Antigravity CLI was not installed where expected.' }}\r\n",
            installed = installed.display().to_string().replace('\'', "''"),
        );
        std::fs::write(&script, text)?;
        Ok(script)
    } else {
        let commands = format!(
            "echo 'Installing the Antigravity CLI from {ANTIGRAVITY_INSTALL_SH} ...'\n\
             curl -fsSL {ANTIGRAVITY_INSTALL_SH} | bash\n\
             if [ -x '{0}' ]; then exec '{0}' {arguments}; fi\n\
             echo 'The Antigravity CLI was not installed where expected.'\n\
             exec \"${{SHELL:-/bin/sh}}\"\n",
            shell_quoted(&installed.display().to_string()),
        );
        write_script(folder, "install-antigravity", &commands)
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
    fn reads_opencode_versions() {
        assert_eq!(major_version("opencode v2.3.0\n"), Some(2));
        assert_eq!(major_version("1.19.5\n"), Some(1));
        assert_eq!(major_version(""), None);
        assert_eq!(major_version("unknown"), None);
        assert_eq!(opencode_arguments(Some(1)), Vec::<String>::new());
        assert_eq!(opencode_arguments(Some(2)), ["--standalone"]);
        // Unknown: what STAMP's installer installs.
        assert_eq!(opencode_arguments(None), ["--standalone"]);
    }

    #[test]
    fn prepares_the_folder_for_opencode() {
        let folder = scratch("opencode");
        // Not installed there, so not asked its version: OpenCode 2.
        let script = prepare_opencode(
            &folder,
            "http://127.0.0.1:4321/mcp",
            "secret",
            Path::new("/opt/it's/opencode"),
        )
        .unwrap();
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join(OPENCODE_CONFIG)).unwrap())
                .unwrap();
        let stamp = &config["mcp"]["servers"]["stamp"];
        assert_eq!(stamp["type"], "remote");
        assert_eq!(stamp["url"], "http://127.0.0.1:4321/mcp");
        assert_eq!(stamp["headers"]["Authorization"], "Bearer secret");
        assert_eq!(stamp["oauth"], false);
        assert_eq!(stamp["codemode"], false);
        let rules = config["permissions"].as_array().unwrap();
        let effect = |action: &str| {
            rules
                .iter()
                .rev()
                .find(|rule| rule["action"] == action)
                .map(|rule| rule["effect"].as_str().unwrap().to_string())
        };
        assert_eq!(effect("shell").as_deref(), Some("ask"));
        assert_eq!(effect("edit").as_deref(), Some("ask"));
        assert_eq!(effect("stamp_*").as_deref(), Some("allow"));
        assert_eq!(
            std::fs::read_to_string(folder.join("AGENTS.md")).unwrap(),
            GUIDE
        );
        let text = std::fs::read_to_string(&script).unwrap();
        assert!(text.contains("--standalone"), "{text}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert!(text.contains("'/opt/it'\\''s/opencode'"), "{text}");
            let mode = std::fs::metadata(folder.join(OPENCODE_CONFIG))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o077, 0, "the token must be private");
        }
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn opencode_1_gets_its_own_form_of_configuration() {
        let config = opencode_config("http://127.0.0.1:1/mcp", "t", Some(1));
        let stamp = &config["mcp"]["stamp"];
        assert_eq!(stamp["type"], "remote");
        assert_eq!(stamp["headers"]["Authorization"], "Bearer t");
        assert_eq!(stamp["oauth"], false);
        assert_eq!(stamp["timeout"], OPENCODE_TOOL_TIMEOUT);
        assert_eq!(config["permission"]["bash"], "ask");
        assert!(config.get("permissions").is_none());
    }

    #[test]
    fn opencode_install_script() {
        let folder = scratch("opencode-install");
        let script = prepare_opencode_install(&folder, "http://127.0.0.1:1/mcp", "t").unwrap();
        let text = std::fs::read_to_string(&script).unwrap();
        if cfg!(windows) {
            assert!(text.contains(OPENCODE_LATEST) && text.contains(OPENCODE_RELEASES));
        } else {
            assert!(text.contains(OPENCODE_INSTALL_SH), "{text}");
            assert!(
                text.contains(".opencode/bin/opencode' --standalone"),
                "{text}"
            );
        }
        assert!(folder.join(OPENCODE_CONFIG).is_file());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn prepares_the_folder_for_antigravity() {
        let folder = scratch("antigravity");
        let script = prepare_antigravity(
            &folder,
            "http://127.0.0.1:1234/mcp",
            "secret",
            Path::new("/opt/it's/agy"),
        )
        .unwrap();
        let [dir, file] = ANTIGRAVITY_CONFIG;
        let config = folder.join(dir).join(file);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        let stamp = &json["mcpServers"]["stamp"];
        assert_eq!(stamp["url"], "http://127.0.0.1:1234/mcp");
        assert_eq!(stamp["headers"]["Authorization"], "Bearer secret");
        assert_eq!(stamp["timeoutSeconds"], 900);
        assert_eq!(
            std::fs::read_to_string(folder.join("AGENTS.md")).unwrap(),
            GUIDE
        );
        let text = std::fs::read_to_string(&script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert!(
                text.contains(&format!(
                    "exec '/opt/it'\\''s/agy' --settings {ANTIGRAVITY_SETTINGS}\n"
                )),
                "{text}"
            );
            let mode = std::fs::metadata(&config).unwrap().permissions().mode();
            assert_eq!(mode & 0o077, 0, "the token must be private");
        }
        #[cfg(windows)]
        assert!(text.contains("\"/opt/it's/agy\""), "{text}");
        assert_eq!(
            Agent::Antigravity.command(Path::new("/opt/agy")),
            ("/opt/agy".to_string(), antigravity_arguments())
        );
        let settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(folder.join(ANTIGRAVITY_SETTINGS)).unwrap(),
        )
        .unwrap();
        assert_eq!(settings["permissions"]["allow"][0], "mcp(stamp/*)");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn antigravity_install_script() {
        let folder = scratch("antigravity-install");
        let script = prepare_antigravity_install(&folder, "http://127.0.0.1:1/mcp", "t").unwrap();
        let text = std::fs::read_to_string(&script).unwrap();
        if cfg!(windows) {
            assert!(text.contains(ANTIGRAVITY_INSTALL_PS1), "{text}");
            assert!(text.contains("agy\\bin\\agy.exe"), "{text}");
        } else {
            assert!(text.contains(ANTIGRAVITY_INSTALL_SH), "{text}");
            assert!(
                text.contains(".local/bin/agy' --settings stamp-antigravity-settings.json; fi"),
                "{text}"
            );
        }
        let [dir, file] = ANTIGRAVITY_CONFIG;
        assert!(folder.join(dir).join(file).is_file());
        std::fs::remove_dir_all(&folder).unwrap();
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
