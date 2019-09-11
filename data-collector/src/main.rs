use chrono::{DateTime, Utc};
use lambda_http::{lambda, IntoResponse, Request};
use lambda_runtime::{error::HandlerError, Context};
use serde_json::json;

fn main() {
    lambda!(handler)
}

fn handler(req: Request, _: Context) -> Result<impl IntoResponse, HandlerError> {
    Ok(())
}

struct ToolReport {
    application_name: String,
    git_branch: String,
    git_commit_hash: String,
    tool_name: String,
    tool_output: String,
    output_format: OutputFormat,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    environment: Environment,
    tool_version: String,
}

enum OutputFormat {
    Json,
    PlainText,
}

enum Environment {
    Local,
    CI,
}

