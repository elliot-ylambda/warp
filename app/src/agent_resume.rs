//! Reads the per-pane agent-resume registry written by the claude wrapper / codex hooks.
//! See docs/superpowers/specs/2026-06-20-warp-agent-session-resume-design.md.

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
struct RegistryEntry {
    command: String,
}

fn registry_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(Path::new(&home).join(".warp").join("agent-resume"))
}

fn read_command_in(dir: &Path, uuid_hex: &str) -> Option<String> {
    let path = dir.join(format!("{uuid_hex}.json"));
    let contents = std::fs::read_to_string(path).ok()?;
    let entry: RegistryEntry = serde_json::from_str(&contents).ok()?;
    Some(entry.command)
}

/// Returns the resume command stored for `uuid`, if any. `uuid` is the raw pane UUID bytes;
/// it is hex-encoded (lowercase) to match `$WARP_TERMINAL_SESSION_UUID`.
pub fn read_on_restore_command(uuid: &[u8]) -> Option<String> {
    let dir = registry_dir()?;
    read_command_in(&dir, &hex::encode(uuid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_command_from_registry_file() {
        let dir = std::env::temp_dir().join(format!("agent_resume_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut f = std::fs::File::create(dir.join("deadbeef.json")).unwrap();
        write!(f, r#"{{ "command": "claude --resume abc-123", "cwd": "/tmp" }}"#).unwrap();

        assert_eq!(
            read_command_in(&dir, "deadbeef"),
            Some("claude --resume abc-123".to_string())
        );
        assert_eq!(read_command_in(&dir, "missing"), None);
    }

    #[test]
    fn uuid_hex_is_lowercase() {
        // Must match $WARP_TERMINAL_SESSION_UUID casing.
        assert_eq!(hex::encode([0xAB, 0xCD]), "abcd");
    }
}
