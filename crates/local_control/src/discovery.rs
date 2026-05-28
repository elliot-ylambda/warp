use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::protocol::{ControlActionKind, ErrorCode, InstanceId, PROTOCOL_VERSION};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryRecord {
    pub instance_id: InstanceId,
    pub pid: u32,
    pub channel: String,
    pub build_version: Option<String>,
    pub protocol_version: u16,
    pub started_at_unix_millis: i64,
    pub outside_warp_control_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_broker: Option<String>,
    #[serde(default)]
    pub actions: Vec<ControlActionKind>,
}

impl DiscoveryRecord {
    pub fn disabled(instance_id: InstanceId, pid: u32, channel: impl Into<String>) -> Self {
        Self {
            instance_id,
            pid,
            channel: channel.into(),
            build_version: None,
            protocol_version: PROTOCOL_VERSION,
            started_at_unix_millis: 0,
            outside_warp_control_enabled: false,
            endpoint: None,
            credential_broker: None,
            actions: Vec::new(),
        }
    }

    pub fn is_compatible(&self) -> bool {
        self.protocol_version == PROTOCOL_VERSION
    }

    pub fn is_actionable(&self) -> bool {
        self.outside_warp_control_enabled
            && self.endpoint.is_some()
            && self.credential_broker.is_some()
    }
}

pub fn default_registry_dir() -> PathBuf {
    directories::ProjectDirs::from("dev", "warp", "Warp")
        .map(|dirs| dirs.data_local_dir().join("local-control"))
        .unwrap_or_else(|| std::env::temp_dir().join("warp-local-control"))
}

pub fn broker_token_path(instance_id: &InstanceId) -> PathBuf {
    default_registry_dir().join(format!("{}.broker-token", instance_id.0))
}

pub fn write_broker_token(path: &Path, token: &str) -> io::Result<()> {
    write_owner_only(path, token.as_bytes())
}

pub fn read_broker_token(instance_id: &InstanceId) -> Result<String, ErrorCode> {
    fs::read_to_string(broker_token_path(instance_id))
        .map_err(|_| ErrorCode::UnauthorizedLocalClient)
}

fn write_owner_only(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        set_owner_only_dir(parent)?;
    }
    fs::write(path, bytes)?;
    set_owner_only_file(path)
}

#[cfg(unix)]
fn set_owner_only_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn set_owner_only_dir(_: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_file(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_owner_only_file(_: &Path) -> io::Result<()> {
    Ok(())
}

pub fn write_record(path: &Path, record: &DiscoveryRecord) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(record).map_err(io::Error::other)?;
    write_owner_only(path, &bytes)
}

pub fn read_records(dir: &Path) -> Result<Vec<DiscoveryRecord>, ErrorCode> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut records = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(record) = serde_json::from_str::<DiscoveryRecord>(&contents) else {
            continue;
        };
        if is_pid_live(record.pid) && record.is_compatible() {
            records.push(record);
        }
    }
    Ok(records)
}

pub fn is_pid_live(pid: u32) -> bool {
    if pid == std::process::id() {
        return true;
    }
    let system = sysinfo::System::new_all();
    system.process(sysinfo::Pid::from_u32(pid)).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_record_has_no_actionable_authority() {
        let record = DiscoveryRecord::disabled(InstanceId("i".into()), std::process::id(), "dev");
        assert!(!record.is_actionable());
        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains("127.0.0.1"));
        assert!(!json.contains("credential"));
    }

    #[test]
    fn reads_compatible_live_records_and_ignores_incompatible() {
        let dir = tempfile::tempdir().unwrap();
        let live = DiscoveryRecord {
            instance_id: InstanceId("live".into()),
            pid: std::process::id(),
            channel: "dev".into(),
            build_version: None,
            protocol_version: PROTOCOL_VERSION,
            started_at_unix_millis: 1,
            outside_warp_control_enabled: true,
            endpoint: Some("http://127.0.0.1:1".into()),
            credential_broker: Some("http://127.0.0.1:1/v1/control/credentials".into()),
            actions: vec![ControlActionKind::AppPing],
        };
        let mut incompatible = live.clone();
        incompatible.instance_id = InstanceId("old".into());
        incompatible.protocol_version = 0;
        write_record(&dir.path().join("live.json"), &live).unwrap();
        write_record(&dir.path().join("old.json"), &incompatible).unwrap();
        let records = read_records(dir.path()).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].instance_id, InstanceId("live".into()));
    }
}
