use clap::Args;
use local_control::protocol::InstanceId;
use local_control::selection::InstanceSelector;

#[derive(Debug, Clone, Args, Default)]
#[group(multiple = false)]
pub struct InstanceSelectorArgs {
    /// Select a Warp instance by opaque instance id.
    #[arg(long = "instance", conflicts_with = "pid")]
    pub instance: Option<String>,

    /// Select a Warp instance by process id.
    #[arg(long = "pid", conflicts_with = "instance")]
    pub pid: Option<u32>,
}

impl InstanceSelectorArgs {
    pub fn instance_selector(&self) -> InstanceSelector {
        if let Some(instance) = &self.instance {
            InstanceSelector::Id(InstanceId(instance.clone()))
        } else if let Some(pid) = self.pid {
            InstanceSelector::Pid(pid)
        } else {
            InstanceSelector::Default
        }
    }
}
