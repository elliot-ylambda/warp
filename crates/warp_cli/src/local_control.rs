use std::io::Write as _;
use std::process::ExitCode;

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::aot::{Shell, generate};
use local_control::protocol::{
    Action, ActionKind, ActionMetadata, ControlError, ErrorCode, RequestEnvelope,
};
use local_control::selection::{InstanceSelector, select_instance};
use serde::Serialize;
use serde_json::json;

use crate::agent::OutputFormat;

#[derive(Debug, Parser)]
#[command(
    name = "warpctrl",
    display_name = "warpctrl",
    about = "Control a running local Warp app instance"
)]
pub struct ControlArgs {
    /// Set the output format.
    #[arg(
        long = "output-format",
        global = true,
        value_enum,
        default_value_t = OutputFormat::Pretty,
        env = "WARP_OUTPUT_FORMAT"
    )]
    pub output_format: OutputFormat,

    #[command(subcommand)]
    pub command: ControlCommand,
}

impl ControlArgs {
    pub fn from_env() -> Self {
        let matches = Self::clap_command().get_matches();
        Self::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
    }

    pub fn clap_command() -> clap::Command {
        let bin_name = crate::binary_name().unwrap_or_else(|| "warpctrl".to_owned());
        <Self as CommandFactory>::command()
            .version(crate::version_string())
            .bin_name(bin_name.clone())
            .after_help(color_print::cformat!(
                r#"<bold><underline>Examples:</underline></bold>

  <dim>$</dim> <bold>{bin_name} instance list</bold>

  <dim>$</dim> <bold>{bin_name} tab create</bold>

<bold><underline>Learn more:</underline></bold>
* Use <bold>{bin_name} help</bold> to learn more about each command
"#
            ))
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum ControlCommand {
    /// Inspect local Warp app instances.
    #[command(subcommand)]
    Instance(InstanceCommand),
    /// Inspect a selected local Warp app.
    #[command(subcommand)]
    App(AppCommand),

    /// Control local Warp tabs.
    #[command(subcommand)]
    Tab(TabCommand),

    /// Generate shell completions for your shell to stdout.
    ///
    /// For bash, add the following to ~/.bashrc:
    ///     source <(path/to/warpctrl completions bash)
    ///
    /// For zsh, add the following to ~/.zshrc:
    ///     source <(path/to/warpctrl completions zsh)
    ///
    /// For fish, add the following to ~/.config/fish/config.fish:
    ///     path/to/warpctrl completions fish | source
    ///
    /// For Powershell, add the following to $PROFILE:
    ///     path\to\warpctrl completions powershell | Out-String | Invoke-Expression
    ///
    /// If no shell is provided, this defaults to the shell that Warp was run from.
    #[command(verbatim_doc_comment)]
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
    /// Check that the selected local Warp app responds.
    Ping(TargetArgs),

    /// Print protocol and app version metadata for the selected local Warp app.
    Version(TargetArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum TabCommand {
    /// Create a new terminal tab in the active window.
    Create(TargetArgs),
}

#[derive(Debug, Clone, Args, Default)]
pub struct TargetArgs {
    /// Target a specific local Warp instance id from `warp instance list`.
    #[arg(long = "instance")]
    pub instance: Option<String>,

    /// Target a specific local Warp process id.
    #[arg(long = "pid", conflicts_with = "instance")]
    pub pid: Option<u32>,
}

#[derive(Serialize)]
struct InstanceSummary {
    instance_id: String,
    pid: u32,
    channel: String,
    app_id: String,
    app_version: Option<String>,
    started_at: String,
    endpoint: Option<local_control::discovery::ControlEndpoint>,
    outside_warp_control_enabled: bool,
    actions: Vec<ActionMetadata>,
}

impl From<local_control::discovery::InstanceRecord> for InstanceSummary {
    fn from(record: local_control::discovery::InstanceRecord) -> Self {
        Self {
            instance_id: record.instance_id.0,
            pid: record.pid,
            channel: record.channel,
            app_id: record.app_id,
            app_version: record.app_version,
            started_at: record.started_at.to_rfc3339(),
            endpoint: record.endpoint,
            outside_warp_control_enabled: record.outside_warp_control_enabled,
            actions: record.actions,
        }
    }
}

#[derive(Serialize)]
struct ErrorSummary<'a> {
    ok: bool,
    error: &'a ControlError,
}

pub fn run(args: ControlArgs) -> ExitCode {
    let output_format = args.output_format;
    match run_inner(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if let Err(write_error) = write_control_error(&error, output_format) {
                eprintln!(
                    "error: failed to render local-control error: {}",
                    write_error.message
                );
            }
            ExitCode::FAILURE
        }
    }
}

fn run_inner(args: ControlArgs) -> Result<(), ControlError> {
    let output_format = args.output_format;
    match args.command {
        ControlCommand::Instance(command) => run_instance_command(command, output_format),
        ControlCommand::App(command) => run_app_command(command, output_format),
        ControlCommand::Tab(command) => run_tab_command(command, output_format),
        ControlCommand::Completions { shell } => generate_completions_to_stdout(shell),
    }
}

fn run_instance_command(
    command: InstanceCommand,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match command {
        InstanceCommand::List => {
            let summaries = local_control::discovery::list_instances()
                .into_iter()
                .map(InstanceSummary::from)
                .collect::<Vec<_>>();
            match output_format {
                OutputFormat::Json => write_json(&summaries),
                OutputFormat::Ndjson => {
                    for summary in summaries {
                        write_json_line(&summary)?;
                    }
                    Ok(())
                }
                OutputFormat::Pretty | OutputFormat::Text => {
                    for summary in summaries {
                        let endpoint = summary
                            .endpoint
                            .as_ref()
                            .map(|endpoint| format!("{}:{}", endpoint.host, endpoint.port))
                            .unwrap_or_else(|| "outside_warp_disabled".to_owned());
                        println!(
                            "{}\tpid={}\t{}\t{}",
                            summary.instance_id, summary.pid, summary.channel, endpoint
                        );
                    }
                    Ok(())
                }
            }
        }
    }
}

fn run_app_command(command: AppCommand, output_format: OutputFormat) -> Result<(), ControlError> {
    match command {
        AppCommand::Ping(args) => run_action(args, ActionKind::AppPing, json!({}), output_format),
        AppCommand::Version(args) => {
            run_action(args, ActionKind::AppVersion, json!({}), output_format)
        }
    }
}
fn run_tab_command(command: TabCommand, output_format: OutputFormat) -> Result<(), ControlError> {
    match command {
        TabCommand::Create(args) => {
            run_action(args, ActionKind::TabCreate, json!({}), output_format)
        }
    }
}

fn run_action(
    args: TargetArgs,
    action: ActionKind,
    params: serde_json::Value,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    let records = local_control::discovery::list_instances();
    let selector = instance_selector(args);
    let instance = select_instance(&records, &selector)?;
    let request = RequestEnvelope::new(Action {
        kind: action,
        params,
    });
    let response = local_control::client::send_request(&instance, &request)?;
    let local_control::protocol::ControlResponse::Ok { data } = response.response else {
        return Err(ControlError::new(
            ErrorCode::Internal,
            "local-control request failed without an error payload",
        ));
    };
    match output_format {
        OutputFormat::Json => write_json(&data),
        OutputFormat::Ndjson => write_json_line(&data),
        OutputFormat::Pretty | OutputFormat::Text => write_json(&data),
    }
}

fn instance_selector(args: TargetArgs) -> InstanceSelector {
    if let Some(instance_id) = args.instance {
        return InstanceSelector::Id(local_control::discovery::InstanceId(instance_id));
    }
    if let Some(pid) = args.pid {
        return InstanceSelector::Pid(pid);
    }
    InstanceSelector::Active
}

fn generate_completions_to_stdout(shell: Option<Shell>) -> Result<(), ControlError> {
    let shell = shell.or_else(Shell::from_env).ok_or_else(|| {
        ControlError::new(
            ErrorCode::InvalidParams,
            "could not determine shell from environment; provide a shell argument",
        )
    })?;
    let mut cmd = ControlArgs::clap_command();
    let bin_name = crate::binary_name().unwrap_or_else(|| "warpctrl".to_owned());
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
    Ok(())
}

#[cfg(test)]
fn generate_completion_string(shell: Shell) -> Result<String, ControlError> {
    let mut cmd = ControlArgs::clap_command();
    let mut output = Vec::new();
    generate(shell, &mut cmd, "warpctrl", &mut output);
    String::from_utf8(output).map_err(|err| {
        ControlError::with_details(
            ErrorCode::Internal,
            "failed to render local-control completions",
            err.to_string(),
        )
    })
}

fn write_control_error(
    error: &ControlError,
    output_format: OutputFormat,
) -> Result<(), ControlError> {
    match output_format {
        OutputFormat::Json => write_json(&ErrorSummary { ok: false, error }),
        OutputFormat::Ndjson => write_json_line(&ErrorSummary { ok: false, error }),
        OutputFormat::Pretty | OutputFormat::Text => {
            eprintln!("error: {}: {}", error.code, error.message);
            if let Some(details) = &error.details {
                eprintln!("details: {details}");
            }
            Ok(())
        }
    }
}

fn write_json(value: &impl Serialize) -> Result<(), ControlError> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value).map_err(write_error)?;
    writeln!(&mut lock).map_err(write_error)?;
    Ok(())
}
fn write_json_line(value: &impl Serialize) -> Result<(), ControlError> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer(&mut lock, value).map_err(write_error)?;
    writeln!(&mut lock).map_err(write_error)?;
    Ok(())
}
fn write_error(error: impl std::error::Error) -> ControlError {
    ControlError::with_details(
        ErrorCode::Internal,
        "failed to write local-control output",
        error.to_string(),
    )
}

#[cfg(test)]
#[path = "local_control_tests.rs"]
mod tests;
