use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::protocol::PROTOCOL_VERSION;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryRecord {
    pub instance_id: String,
    pub pid: u32,
    pub channel: String,
    pub app_version: String,
    pub protocol_version: u16,
    pub started_at_unix_ms: u64,
    pub outside_warp_control_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub implemented_actions: Vec<String>,
}

impl DiscoveryRecord {
    pub fn disabled(instance_id: impl Into<String>, pid: u32) -> Self {
        Self {
            instance_id: instance_id.into(),
            pid,
            channel: String::new(),
            app_version: String::new(),
            protocol_version: PROTOCOL_VERSION,
            started_at_unix_ms: 0,
            outside_warp_control_enabled: false,
            endpoint: None,
            implemented_actions: Vec::new(),
        }
    }

    pub fn is_actionable(&self) -> bool {
        self.outside_warp_control_enabled && self.endpoint.is_some()
    }

    pub fn is_compatible(&self) -> bool {
        self.protocol_version == PROTOCOL_VERSION
    }

    pub fn is_stale(&self) -> bool {
        self.pid == 0 || !self.is_compatible()
    }
}

pub fn default_registry_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("warp").join("local-control"))
}

pub fn write_record(dir: &Path, record: &DiscoveryRecord) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{}.json", record.instance_id));
    let bytes = serde_json::to_vec_pretty(record).map_err(io::Error::other)?;
    fs::write(&path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(path)
}

pub fn read_records(dir: &Path) -> io::Result<Vec<DiscoveryRecord>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read(entry.path())?;
        let record = serde_json::from_slice::<DiscoveryRecord>(&bytes).map_err(io::Error::other)?;
        if record.is_stale() {
            let _ = fs::remove_file(entry.path());
            continue;
        }
        records.push(record);
    }
    Ok(records)
}

pub fn compatible_actionable_records(records: &[DiscoveryRecord]) -> Vec<DiscoveryRecord> {
    records
        .iter()
        .filter(|record| record.is_compatible() && record.is_actionable())
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_record_has_no_actionable_endpoint() {
        let record = DiscoveryRecord::disabled("instance", 123);
        assert!(!record.is_actionable());
        let json = serde_json::to_value(&record).unwrap();
        assert!(json.get("token").is_none());
        assert!(json.get("credential").is_none());
        assert!(json.get("endpoint").is_none());
    }

    #[test]
    fn read_records_prunes_incompatible_records() {
        let dir = tempfile::tempdir().unwrap();
        let mut record = DiscoveryRecord::disabled("old", 1);
        record.protocol_version = PROTOCOL_VERSION + 1;
        write_record(dir.path(), &record).unwrap();
        assert!(read_records(dir.path()).unwrap().is_empty());
        assert!(!dir.path().join("old.json").exists());
    }

    #[test]
    fn actionable_records_require_enabled_endpoint() {
        let disabled = DiscoveryRecord::disabled("disabled", 1);
        let enabled = DiscoveryRecord {
            instance_id: "enabled".to_owned(),
            pid: 1,
            channel: "dev".to_owned(),
            app_version: "1".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            started_at_unix_ms: 1,
            outside_warp_control_enabled: true,
            endpoint: Some("http://127.0.0.1:1".to_owned()),
            implemented_actions: vec!["app.ping".to_owned()],
        };
        assert_eq!(
            compatible_actionable_records(&[disabled, enabled.clone()]),
            vec![enabled]
        );
    }
}
