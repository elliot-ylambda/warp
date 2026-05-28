use std::io;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::aot::{Shell, generate};
use local_control::client::LocalControlClient;
use local_control::discovery::{compatible_actionable_records, default_registry_dir, read_records};
use local_control::protocol::{
    ControlAction, ControlError, CredentialRequest, ErrorCode, RequestEnvelope, ResponseEnvelope,
    TargetSelector, WindowSelector,
};
use local_control::selection::{InstanceSelector, select_instance};
use serde_json::{Value, json};

use crate::agent::OutputFormat;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "warpctrl",
    display_name = "warpctrl",
    about = "Control a running local Warp app instance through the allowlisted local-control protocol"
)]
pub struct LocalControlArgs {
    /// Set the output format.
    #[arg(long = "output-format", global = true, value_enum, default_value_t = OutputFormat::Pretty)]
    pub output_format: OutputFormat,

    #[command(subcommand)]
    pub command: LocalControlCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum LocalControlCommand {
    /// Discover running Warp instances.
    #[command(subcommand)]
    Instance(InstanceCommand),
    /// App metadata and health commands.
    #[command(subcommand)]
    App(AppCommand),
    /// Tab commands.
    #[command(subcommand)]
    Tab(TabCommand),
    /// Generate shell completions for warpctrl.
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: Option<Shell>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum InstanceCommand {
    /// List locally discoverable Warp instances.
    List,
}

#[derive(Debug, Clone, Subcommand)]
pub enum AppCommand {
    /// Ping the selected Warp instance.
    Ping(InstanceSelectionArgs),
    /// Return version metadata for the selected Warp instance.
    Version(InstanceSelectionArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum TabCommand {
    /// Create a new terminal tab in the selected Warp instance's active window.
    Create(TabCreateArgs),
}

#[derive(Debug, Clone, Default, clap::Args)]
pub struct InstanceSelectionArgs {
    /// Select a Warp instance by opaque instance ID.
    #[arg(long = "instance", conflicts_with = "pid")]
    pub instance: Option<String>,
    /// Select a Warp instance by process ID.
    #[arg(long = "pid", conflicts_with = "instance")]
    pub pid: Option<u32>,
}

impl InstanceSelectionArgs {
    pub fn selector(&self) -> InstanceSelector {
        if let Some(instance) = &self.instance {
            InstanceSelector::Id(instance.clone())
        } else if let Some(pid) = self.pid {
            InstanceSelector::Pid(pid)
        } else {
            InstanceSelector::Any
        }
    }
}

#[derive(Debug, Clone, Default, clap::Args)]
pub struct TabCreateArgs {
    #[clap(flatten)]
    pub instance: InstanceSelectionArgs,
    /// Target the active window. Concrete window selectors are reserved for a later slice.
    #[arg(long = "window", value_parser = ["active"], default_value = "active")]
    pub window: String,
}

pub fn run_from_env() -> Result<()> {
    let args = LocalControlArgs::parse();
    let output = execute(args)?;
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

pub fn execute(args: LocalControlArgs) -> Result<String> {
    match args.command {
        LocalControlCommand::Completions { shell } => generate_completions(shell),
        LocalControlCommand::Instance(InstanceCommand::List) => instance_list(args.output_format),
        LocalControlCommand::App(AppCommand::Ping(selection)) => execute_remote(
            ControlAction::AppPing,
            selection.selector(),
            TargetSelector::default(),
            args.output_format,
        ),
        LocalControlCommand::App(AppCommand::Version(selection)) => execute_remote(
            ControlAction::AppVersion,
            selection.selector(),
            TargetSelector::default(),
            args.output_format,
        ),
        LocalControlCommand::Tab(TabCommand::Create(tab_args)) => execute_remote(
            ControlAction::TabCreate,
            tab_args.instance.selector(),
            TargetSelector {
                window: Some(WindowSelector::Active),
                ..Default::default()
            },
            args.output_format,
        ),
    }
}

fn generate_completions(shell: Option<Shell>) -> Result<String> {
    let shell = shell
        .or_else(Shell::from_env)
        .context("could not determine shell from environment; provide a shell argument")?;
    let mut command = LocalControlArgs::command();
    generate(shell, &mut command, "warpctrl", &mut io::stdout());
    Ok(String::new())
}

fn instance_list(output_format: OutputFormat) -> Result<String> {
    let records = load_actionable_records()?;
    render_output(output_format, json!({ "instances": records }))
}

fn execute_remote(
    action: ControlAction,
    selector: InstanceSelector,
    target: TargetSelector,
    output_format: OutputFormat,
) -> Result<String> {
    let records = load_actionable_records()?;
    let instance = select_instance(&records, &selector).map_err(anyhow_for_control_error)?;
    let endpoint = instance.endpoint.clone().ok_or_else(|| {
        anyhow_for_control_error(ControlError::new(
            ErrorCode::NoInstance,
            "selected Warp instance does not expose an actionable endpoint",
        ))
    })?;
    let client = LocalControlClient::new(endpoint);
    let credential = client.request_credential(&CredentialRequest::outside_warp(action.clone()))?;
    let mut request = RequestEnvelope::new(action);
    request.target = target;
    let response = client.send_request(&credential, &request)?;
    render_response(output_format, response)
}

fn load_actionable_records() -> Result<Vec<local_control::discovery::DiscoveryRecord>> {
    let Some(dir) = default_registry_dir() else {
        return Ok(Vec::new());
    };
    Ok(compatible_actionable_records(&read_records(&dir)?))
}

fn render_response(output_format: OutputFormat, response: ResponseEnvelope) -> Result<String> {
    if !response.ok
        && let Some(error) = response.error
    {
        return Err(anyhow_for_control_error(error));
    }
    render_output(output_format, serde_json::to_value(response)?)
}

fn render_output(output_format: OutputFormat, value: Value) -> Result<String> {
    match output_format {
        OutputFormat::Json | OutputFormat::Ndjson => Ok(serde_json::to_string(&value)?),
        OutputFormat::Pretty | OutputFormat::Text => Ok(render_pretty(&value)),
    }
}

fn render_pretty(value: &Value) -> String {
    if let Some(instances) = value.get("instances").and_then(Value::as_array) {
        if instances.is_empty() {
            return "No compatible Warp instances found.".to_owned();
        }
        return instances
            .iter()
            .map(|instance| {
                format!(
                    "{}\tpid={}\tchannel={}\tprotocol={}\tactions={}",
                    instance["instance_id"].as_str().unwrap_or("unknown"),
                    instance["pid"].as_u64().unwrap_or_default(),
                    instance["channel"].as_str().unwrap_or("unknown"),
                    instance["protocol_version"].as_u64().unwrap_or_default(),
                    instance["implemented_actions"]
                        .as_array()
                        .map(|actions| actions
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(","))
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn anyhow_for_control_error(error: ControlError) -> anyhow::Error {
    anyhow::anyhow!("{}: {}", error.code.as_str(), error.message)
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn parses_first_slice_commands() {
        assert!(matches!(
            LocalControlArgs::try_parse_from(["warpctrl", "instance", "list"])
                .unwrap()
                .command,
            LocalControlCommand::Instance(InstanceCommand::List)
        ));
        assert!(matches!(
            LocalControlArgs::try_parse_from(["warpctrl", "app", "ping", "--pid", "123"])
                .unwrap()
                .command,
            LocalControlCommand::App(AppCommand::Ping(_))
        ));
        assert!(matches!(
            LocalControlArgs::try_parse_from([
                "warpctrl",
                "tab",
                "create",
                "--instance",
                "abc",
                "--window",
                "active",
            ])
            .unwrap()
            .command,
            LocalControlCommand::Tab(TabCommand::Create(_))
        ));
    }

    #[test]
    fn rejects_future_commands_not_exposed_in_foundation_parser() {
        assert!(LocalControlArgs::try_parse_from(["warpctrl", "window", "list"]).is_err());
        assert!(LocalControlArgs::try_parse_from(["warpctrl", "input", "run", "pwd"]).is_err());
    }

    #[test]
    fn rejects_conflicting_instance_selectors() {
        assert!(
            LocalControlArgs::try_parse_from([
                "warpctrl",
                "app",
                "version",
                "--instance",
                "a",
                "--pid",
                "1",
            ])
            .is_err()
        );
    }

    #[test]
    fn renders_no_discovery_record_as_no_instance_output() {
        let output = render_output(OutputFormat::Pretty, json!({ "instances": [] })).unwrap();
        assert_eq!(output, "No compatible Warp instances found.");
    }
}
