use crate::discovery::DiscoveryRecord;
use crate::protocol::{ErrorCode, InstanceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstanceSelector {
    Default,
    Id(InstanceId),
    Pid(u32),
}

pub fn select_instance<'a>(
    records: &'a [DiscoveryRecord],
    selector: &InstanceSelector,
) -> Result<&'a DiscoveryRecord, ErrorCode> {
    let compatible: Vec<&DiscoveryRecord> = records
        .iter()
        .filter(|record| record.is_compatible() && record.is_actionable())
        .collect();
    match selector {
        InstanceSelector::Default => match compatible.as_slice() {
            [] => Err(ErrorCode::NoInstance),
            [record] => Ok(record),
            _ => Err(ErrorCode::AmbiguousInstance),
        },
        InstanceSelector::Id(id) => {
            let matches: Vec<&DiscoveryRecord> = compatible
                .into_iter()
                .filter(|record| &record.instance_id == id)
                .collect();
            exactly_one(matches)
        }
        InstanceSelector::Pid(pid) => {
            let matches: Vec<&DiscoveryRecord> = compatible
                .into_iter()
                .filter(|record| record.pid == *pid)
                .collect();
            exactly_one(matches)
        }
    }
}

fn exactly_one(matches: Vec<&DiscoveryRecord>) -> Result<&DiscoveryRecord, ErrorCode> {
    match matches.as_slice() {
        [] => Err(ErrorCode::NoInstance),
        [record] => Ok(record),
        _ => Err(ErrorCode::AmbiguousInstance),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ControlActionKind, PROTOCOL_VERSION};

    fn record(id: &str, pid: u32) -> DiscoveryRecord {
        DiscoveryRecord {
            instance_id: InstanceId(id.into()),
            pid,
            channel: "dev".into(),
            build_version: None,
            protocol_version: PROTOCOL_VERSION,
            started_at_unix_millis: 0,
            outside_warp_control_enabled: true,
            endpoint: Some("http://127.0.0.1:1".into()),
            credential_broker: Some("http://127.0.0.1:1/v1/control/credentials".into()),
            actions: vec![ControlActionKind::AppPing],
        }
    }

    #[test]
    fn default_selection_handles_zero_one_many() {
        assert_eq!(
            select_instance(&[], &InstanceSelector::Default),
            Err(ErrorCode::NoInstance)
        );
        let one = vec![record("a", 1)];
        assert_eq!(
            select_instance(&one, &InstanceSelector::Default)
                .unwrap()
                .instance_id
                .0,
            "a"
        );
        let many = vec![record("a", 1), record("b", 2)];
        assert_eq!(
            select_instance(&many, &InstanceSelector::Default),
            Err(ErrorCode::AmbiguousInstance)
        );
    }

    #[test]
    fn explicit_id_and_pid_must_resolve_exactly() {
        let records = vec![record("a", 11), record("b", 12)];
        assert_eq!(
            select_instance(&records, &InstanceSelector::Id(InstanceId("b".into())))
                .unwrap()
                .pid,
            12
        );
        assert_eq!(
            select_instance(&records, &InstanceSelector::Pid(11))
                .unwrap()
                .instance_id
                .0,
            "a"
        );
        assert_eq!(
            select_instance(&records, &InstanceSelector::Pid(99)),
            Err(ErrorCode::NoInstance)
        );
    }
}
