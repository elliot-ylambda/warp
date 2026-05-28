use crate::discovery::DiscoveryRecord;
use crate::protocol::{ControlError, ErrorCode};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstanceSelector {
    Any,
    Id(String),
    Pid(u32),
}

pub fn select_instance(
    records: &[DiscoveryRecord],
    selector: &InstanceSelector,
) -> Result<DiscoveryRecord, ControlError> {
    let matches: Vec<DiscoveryRecord> = match selector {
        InstanceSelector::Any => records.to_vec(),
        InstanceSelector::Id(id) => records
            .iter()
            .filter(|record| &record.instance_id == id)
            .cloned()
            .collect(),
        InstanceSelector::Pid(pid) => records
            .iter()
            .filter(|record| &record.pid == pid)
            .cloned()
            .collect(),
    };

    match matches.as_slice() {
        [] => Err(ControlError::new(
            match selector {
                InstanceSelector::Any => ErrorCode::NoInstance,
                InstanceSelector::Id(_) | InstanceSelector::Pid(_) => ErrorCode::StaleTarget,
            },
            "no compatible Warp instance matched the selector",
        )),
        [record] => Ok(record.clone()),
        _ => Err(ControlError::new(
            ErrorCode::AmbiguousInstance,
            "multiple compatible Warp instances matched the selector",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::DiscoveryRecord;
    use crate::protocol::PROTOCOL_VERSION;

    fn record(id: &str, pid: u32) -> DiscoveryRecord {
        DiscoveryRecord {
            instance_id: id.to_owned(),
            pid,
            channel: "dev".to_owned(),
            app_version: "1".to_owned(),
            protocol_version: PROTOCOL_VERSION,
            started_at_unix_ms: 1,
            outside_warp_control_enabled: true,
            endpoint: Some(format!("http://127.0.0.1:{pid}")),
            implemented_actions: vec![],
        }
    }

    #[test]
    fn zero_one_multiple_instance_selection() {
        assert_eq!(
            select_instance(&[], &InstanceSelector::Any)
                .unwrap_err()
                .code,
            ErrorCode::NoInstance
        );
        assert_eq!(
            select_instance(&[record("a", 1)], &InstanceSelector::Any)
                .unwrap()
                .instance_id,
            "a"
        );
        assert_eq!(
            select_instance(&[record("a", 1), record("b", 2)], &InstanceSelector::Any)
                .unwrap_err()
                .code,
            ErrorCode::AmbiguousInstance
        );
    }

    #[test]
    fn explicit_id_and_pid_must_resolve_exactly() {
        let records = [record("a", 42), record("b", 43)];
        assert_eq!(
            select_instance(&records, &InstanceSelector::Id("a".to_owned()))
                .unwrap()
                .pid,
            42
        );
        assert_eq!(
            select_instance(&records, &InstanceSelector::Pid(43))
                .unwrap()
                .instance_id,
            "b"
        );
        assert_eq!(
            select_instance(&records, &InstanceSelector::Id("missing".to_owned()))
                .unwrap_err()
                .code,
            ErrorCode::StaleTarget
        );
    }
}
