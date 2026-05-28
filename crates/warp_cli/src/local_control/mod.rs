use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::aot::{Shell, generate};
use local_control::client::Client;
use local_control::protocol::{ControlActionKind, ErrorCode, ResponseEnvelope};
use local_control::selection::InstanceSelector;

use crate::agent::OutputFormat;

pub mod output;
pub mod selectors;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "warpctrl",
    display_name = "warpctrl",
    about = "Control a running local Warp app instance through the allowlisted local-control protocol"
)]
pub struct WarpCtrlArgs {
    #[arg(long = "output-format", global = true, value_enum, default_value_t = OutputFormat::Pretty)]
    pub output_format: OutputFormat,

    #[command(subcommand)]
    pub command: WarpCtrlCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum WarpCtrlCommand {
    /// Discover local Warp app instances.
    #[command(subcommand)]
    Instance(InstanceCommand),
    /// Minimal app metadata and health commands.
    #[command(subcommand)]
    App(AppCommand),
    /// Tab commands implemented by the foundation slice.
    #[command(subcommand)]
    Tab(TabCommand),
    /// Generate shell completions for warpctrl.
    Completions {
        #[arg(value_enum)]
        shell: Option<Shell>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum InstanceCommand {
    /// List compatible local Warp instances.
    List,
}

#[derive(Debug, Clone, Subcommand)]
pub enum AppCommand {
    /// Ping the selected Warp instance.
    Ping(selectors::InstanceSelectorArgs),
    /// Return app and protocol version metadata for the selected Warp instance.
    Version(selectors::InstanceSelectorArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum TabCommand {
    /// Create a terminal tab in the active window of the selected Warp instance.
    Create(selectors::InstanceSelectorArgs),
}

pub fn run_from_env() -> anyhow::Result<()> {
    let args = WarpCtrlArgs::parse();
    run(args)
}

pub fn run(args: WarpCtrlArgs) -> anyhow::Result<()> {
    match args.command {
        WarpCtrlCommand::Completions { shell } => generate_completions(shell),
        WarpCtrlCommand::Instance(InstanceCommand::List) => {
            let client = Client::default();
            let records = client.instances().unwrap_or_default();
            output::print_value(
                args.output_format,
                &local_control::client::Client::render_instances_json(&records),
            )
        }
        WarpCtrlCommand::App(AppCommand::Ping(selector)) => run_action(
            args.output_format,
            selector.instance_selector(),
            ControlActionKind::AppPing,
        ),
        WarpCtrlCommand::App(AppCommand::Version(selector)) => run_action(
            args.output_format,
            selector.instance_selector(),
            ControlActionKind::AppVersion,
        ),
        WarpCtrlCommand::Tab(TabCommand::Create(selector)) => run_action(
            args.output_format,
            selector.instance_selector(),
            ControlActionKind::TabCreate,
        ),
    }
}

fn run_action(
    output_format: OutputFormat,
    selector: InstanceSelector,
    action: ControlActionKind,
) -> anyhow::Result<()> {
    let client = Client::default();
    let response = match client.send(&selector, action) {
        Ok(response) => response,
        Err(code) => response_for_error(action, code),
    };
    output::print_response(output_format, &response)?;
    if response.ok {
        Ok(())
    } else {
        let code = response
            .error
            .as_ref()
            .map(|error| error.code.to_string())
            .unwrap_or_else(|| "unknown_error".to_owned());
        anyhow::bail!("warpctrl request failed: {code}")
    }
}

fn response_for_error(action: ControlActionKind, code: ErrorCode) -> ResponseEnvelope {
    let message = match code {
        ErrorCode::NoInstance => "no running Warp instance found for local control",
        ErrorCode::AmbiguousInstance => {
            "multiple compatible Warp instances found; pass --instance or --pid"
        }
        ErrorCode::LocalControlDisabled => "outside-Warp local control is disabled",
        ErrorCode::ExecutionContextNotAllowed => {
            "inside-Warp local control is not supported in this foundation slice"
        }
        _ => "local-control request failed",
    };
    ResponseEnvelope::error(action.name(), code, message)
}

fn generate_completions(shell: Option<Shell>) -> anyhow::Result<()> {
    let shell = match shell.or_else(Shell::from_env) {
        Some(shell) => shell,
        None => anyhow::bail!(
            "Could not determine shell from environment. Please provide a shell argument."
        ),
    };
    let mut cmd = WarpCtrlArgs::command();
    generate(shell, &mut cmd, "warpctrl", &mut std::io::stdout());
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::*;

    #[test]
    fn parses_first_slice_commands() {
        WarpCtrlArgs::parse_from(["warpctrl", "instance", "list"]);
        WarpCtrlArgs::parse_from(["warpctrl", "app", "ping"]);
        WarpCtrlArgs::parse_from(["warpctrl", "app", "version", "--pid", "123"]);
        WarpCtrlArgs::parse_from(["warpctrl", "tab", "create", "--instance", "abc"]);
        WarpCtrlArgs::parse_from(["warpctrl", "completions", "zsh"]);
    }

    #[test]
    fn rejects_future_commands_not_in_first_slice() {
        assert!(WarpCtrlArgs::try_parse_from(["warpctrl", "window", "list"]).is_err());
        assert!(WarpCtrlArgs::try_parse_from(["warpctrl", "input", "run", "pwd"]).is_err());
    }

    #[test]
    fn instance_selector_flags_conflict() {
        assert!(
            WarpCtrlArgs::try_parse_from([
                "warpctrl",
                "app",
                "ping",
                "--instance",
                "a",
                "--pid",
                "1",
            ])
            .is_err()
        );
    }
}
