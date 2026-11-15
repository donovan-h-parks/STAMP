//! STAMP as a Claude Desktop extension: an MCP bundle (`.mcpb`), a ZIP
//! file holding `manifest.json`, an icon and the `stamp-mcp` bridge for
//! this computer. STAMP builds it from the bridge installed beside it and
//! hands it to Claude Desktop, which shows its own install dialog.
//!
//! The manifest follows the MCPB specification, version 0.3 (`binary`
//! server, as in its `calculator-rust` example).

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::tools;

/// File name of the bridge on this platform.
pub fn bridge_name() -> &'static str {
    if cfg!(windows) {
        "stamp-mcp.exe"
    } else {
        "stamp-mcp"
    }
}

/// The bridge installed beside the running STAMP (in `STAMP.app` on
/// macOS, beside `stamp.exe` on Windows), if it is there.
pub fn find_bridge() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let candidate = exe.parent()?.join(bridge_name());
    candidate.is_file().then_some(candidate)
}

/// The platform as Claude Desktop names it (Node's `process.platform`).
fn platform() -> &'static str {
    if cfg!(windows) {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

/// The extension's `manifest.json`.
pub fn manifest() -> serde_json::Value {
    let entry = format!("server/{}", bridge_name());
    let tools: Vec<serde_json::Value> = tools::list()
        .iter()
        .map(|t| json!({"name": t.name, "description": t.description}))
        .collect();
    json!({
        "manifest_version": "0.3",
        "name": "stamp",
        "display_name": "STAMP",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Ask Claude about the data, results and plots open in STAMP, and have it \
                        change the analysis and the plots.",
        "long_description": "Connects Claude to STAMP (statistical analysis of taxonomic and \
            functional profiles) running on this computer. Claude can read the loaded profile and \
            metadata, the results of the current analysis and the plot on screen, show other \
            plots, change plot options and the analysis, and undo its changes. In STAMP, choose \
            Assistant > Share with Claude Desktop to let Claude see the session; nothing is \
            shared until then.",
        "author": {"name": "Donovan Parks and Rob Beiko"},
        "repository": {"type": "git", "url": env!("CARGO_PKG_REPOSITORY")},
        "license": "GPL-3.0-or-later",
        "icon": "icon.png",
        "server": {
            "type": "binary",
            "entry_point": entry,
            "mcp_config": {
                "command": format!("${{__dirname}}/{entry}"),
                "args": [],
                "env": {}
            }
        },
        "tools": tools,
        "compatibility": {"platforms": [platform()]}
    })
}

/// Write the extension, holding `bridge`, to `destination`.
pub fn write(destination: &Path, bridge: &Path) -> std::io::Result<()> {
    let manifest = serde_json::to_string_pretty(&manifest()).unwrap_or_default();
    let bridge_bytes = std::fs::read(bridge)?;
    let icon = include_bytes!("../../assets/icon.png");
    let mut zip = ZipWriter::default();
    zip.add("manifest.json", manifest.as_bytes(), 0o644)?;
    zip.add("icon.png", icon, 0o644)?;
    zip.add(&format!("server/{}", bridge_name()), &bridge_bytes, 0o755)?;
    if let Some(folder) = destination.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(destination, zip.finish())
}

/// Hand the extension to Claude Desktop by opening it as the system
/// opens files of its type. Linux has no such association for Claude
/// Desktop: false there, and the user installs it from Claude Desktop's
/// Extensions settings.
pub fn open(bundle: &Path) -> Result<bool, String> {
    let mut command = if cfg!(windows) {
        let mut command = std::process::Command::new("cmd");
        command.args(["/c", "start", ""]).arg(bundle);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = std::process::Command::new("open");
        command.arg(bundle);
        command
    } else {
        return Ok(false);
    };
    command
        .spawn()
        .map(|_| true)
        .map_err(|e| format!("Could not open {}: {e}", bundle.display()))
}

/// A ZIP archive built in memory: deflated entries with Unix permissions
/// (so that the bridge stays executable once unpacked).
#[derive(Default)]
struct ZipWriter {
    bytes: Vec<u8>,
    central: Vec<u8>,
    entries: u16,
}

impl ZipWriter {
    fn add(&mut self, name: &str, data: &[u8], mode: u32) -> std::io::Result<()> {
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        let size = |n: usize| {
            u32::try_from(n).map_err(|_| std::io::Error::other("file too large for a ZIP"))
        };
        let (crc, compressed_size, size) = (crc.sum(), size(compressed.len())?, size(data.len())?);
        let offset = size_u32(self.bytes.len())?;
        let name_length = u16::try_from(name.len()).map_err(std::io::Error::other)?;
        // 1980-01-01 00:00, the earliest date a ZIP can hold.
        let (time, date) = (0u16, 0x21u16);

        let local = &mut self.bytes;
        local.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        local.extend_from_slice(&20u16.to_le_bytes()); // version needed
        local.extend_from_slice(&0u16.to_le_bytes()); // flags
        local.extend_from_slice(&8u16.to_le_bytes()); // deflate
        local.extend_from_slice(&time.to_le_bytes());
        local.extend_from_slice(&date.to_le_bytes());
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&compressed_size.to_le_bytes());
        local.extend_from_slice(&size.to_le_bytes());
        local.extend_from_slice(&name_length.to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes()); // extra field
        local.extend_from_slice(name.as_bytes());
        local.extend_from_slice(&compressed);

        let central = &mut self.central;
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&((3u16 << 8) | 20).to_le_bytes()); // made by Unix
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&8u16.to_le_bytes());
        central.extend_from_slice(&time.to_le_bytes());
        central.extend_from_slice(&date.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&compressed_size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&name_length.to_le_bytes());
        central.extend_from_slice(&[0; 8]); // extra, comment, disk, internal attributes
        central.extend_from_slice(&((0o100_000 | mode) << 16).to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
        self.entries += 1;
        Ok(())
    }

    fn finish(mut self) -> Vec<u8> {
        let start = self.bytes.len() as u32;
        let length = self.central.len() as u32;
        self.bytes.append(&mut self.central);
        self.bytes.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        self.bytes.extend_from_slice(&[0; 4]); // disk numbers
        self.bytes.extend_from_slice(&self.entries.to_le_bytes());
        self.bytes.extend_from_slice(&self.entries.to_le_bytes());
        self.bytes.extend_from_slice(&length.to_le_bytes());
        self.bytes.extend_from_slice(&start.to_le_bytes());
        self.bytes.extend_from_slice(&0u16.to_le_bytes()); // comment
        self.bytes
    }
}

fn size_u32(n: usize) -> std::io::Result<u32> {
    u32::try_from(n).map_err(|_| std::io::Error::other("archive too large for a ZIP"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_names_the_bridge_and_every_tool() {
        let manifest = manifest();
        assert_eq!(manifest["manifest_version"], "0.3");
        for field in ["name", "version", "description", "author", "server"] {
            assert!(!manifest[field].is_null(), "{field} is required");
        }
        let server = &manifest["server"];
        assert_eq!(server["type"], "binary");
        assert_eq!(
            server["mcp_config"]["command"],
            format!("${{__dirname}}/server/{}", bridge_name())
        );
        assert_eq!(server["entry_point"], format!("server/{}", bridge_name()));
        assert_eq!(
            manifest["tools"].as_array().unwrap().len(),
            tools::list().len()
        );
    }

    #[test]
    fn writes_a_zip_with_the_bridge_executable() {
        let folder = std::env::temp_dir().join(format!("stamp-bundle-{}", std::process::id()));
        let bridge = folder.join("fake-bridge");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(&bridge, b"#!/bin/sh\necho hello\n").unwrap();
        let bundle = folder.join("stamp.mcpb");
        write(&bundle, &bridge).unwrap();
        let bytes = std::fs::read(&bundle).unwrap();
        assert!(bytes.starts_with(b"PK\x03\x04"));
        // The end-of-central-directory record says three entries.
        let end = bytes.len() - 22;
        assert_eq!(&bytes[end..end + 4], b"PK\x05\x06");
        assert_eq!(u16::from_le_bytes([bytes[end + 10], bytes[end + 11]]), 3);
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("manifest.json") && text.contains(&format!("server/{}", bridge_name()))
        );
        if let Ok(keep) = std::env::var("STAMP_KEEP_BUNDLE") {
            std::fs::copy(&bundle, keep).unwrap();
        }
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
