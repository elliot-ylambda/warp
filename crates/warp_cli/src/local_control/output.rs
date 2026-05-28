use serde_json::Value;

use crate::agent::OutputFormat;
use local_control::protocol::ResponseEnvelope;

pub fn print_response(format: OutputFormat, response: &ResponseEnvelope) -> anyhow::Result<()> {
    let value = serde_json::to_value(response)?;
    print_value(format, &value)
}

pub fn print_value(format: OutputFormat, value: &Value) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(value)?),
        OutputFormat::Ndjson => println!("{}", serde_json::to_string(value)?),
        OutputFormat::Pretty | OutputFormat::Text => println_pretty(value),
    }
    Ok(())
}

fn println_pretty(value: &Value) {
    if let Some(instances) = value.get("instances").and_then(Value::as_array) {
        if instances.is_empty() {
            println!("No compatible Warp instances found.");
        } else {
            for instance in instances {
                let id = instance
                    .get("instance_id")
                    .and_then(Value::as_str)
                    .unwrap_or("<unknown>");
                let pid = instance
                    .get("pid")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                let channel = instance
                    .get("channel")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let enabled = instance
                    .get("outside_warp_control_enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                println!("{id}\tpid={pid}\tchannel={channel}\toutside_warp_enabled={enabled}");
            }
        }
        return;
    }

    if value.get("ok").and_then(Value::as_bool) == Some(false) {
        let code = value
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("unknown_error");
        let message = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("request failed");
        println!("error: {code}: {message}");
        return;
    }

    if let Some(result) = value.get("result") {
        println!("{result}");
    } else {
        println!("{value}");
    }
}
