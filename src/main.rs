use crate::runner::{CaptureError, CapturedOutput};
use clap::Parser;
use std::process::exit;

mod quote;
mod runner;
mod tap_stream;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// URL of the ntfy server and topic, e.g. https://ntfy.sh/topic
    #[arg(short = 'n', long = "ntfy-url", env = "NTFY_URL", alias = "url")]
    url: String,

    /// Access token to use with ntfy
    #[arg(short, long, env = "NTFY_TOKEN")]
    token: Option<String>,

    /// User to use with ntfy
    #[arg(
        short,
        long,
        env = "NTFY_USER",
        conflicts_with = "token",
        requires = "password"
    )]
    user: Option<String>,

    /// Password to use with nfty
    #[arg(
        short,
        long,
        env = "NTFY_PASSWORD",
        conflicts_with = "token",
        requires = "user"
    )]
    password: Option<String>,

    /// Notify even when the output is empty
    #[arg(short = 'N', long, env = "NTFY_ALWAYS_NOTIFY")]
    always_notify: bool,

    /// Notify only when command fails
    #[arg(
        short = 'o',
        long,
        env = "NTFY_FAILURE_ONLY",
        conflicts_with = "always_notify"
    )]
    only_failures: bool,

    /// Message title, will be prefixed with "Success" or "Failure".
    /// Defaults to command line.
    #[arg(short = 'T', long, env = "NTFY_TITLE")]
    title: Option<String>,

    /// Message title upon successful executions
    #[arg(short = 's', long, env = "NTFY_SUCCESS_TITLE")]
    success_title: Option<String>,

    /// Message title upon failed executions
    #[arg(short = 'f', long, env = "NTFY_FAILURE_TITLE")]
    failure_title: Option<String>,

    /// Message priority upon successful executions
    #[arg(short = 'S', long, env = "NTFY_SUCCESS_PRIORITY")]
    success_priority: Option<String>,

    /// Message priority upon failed executions
    #[arg(short = 'F', long, env = "NTFY_FAILURE_PRIORITY")]
    failure_priority: Option<String>,

    /// Message tags/emojis upon successful executions
    #[arg(short = 'a', long, env = "NTFY_SUCCESS_TAGS")]
    success_tags: Option<String>,

    /// Message tags/emojis upon failed executions
    #[arg(short = 'A', long, env = "NTFY_FAILURE_TAGS")]
    failure_tags: Option<String>,

    /// An optional email for ntfy to notify
    #[arg(short, long, env = "NTFY_EMAIL")]
    email: Option<String>,

    /// URL to icon to display in notification
    #[arg(short, long, env = "NTFY_ICON")]
    icon: Option<String>,

    /// The command line to execute (no shell used).
    /// If shell is desired, pass `bash -c 'command line'`.
    #[arg(trailing_var_arg = true, required = true)]
    cmdline: Vec<String>,
}

fn format_post_body(output: CapturedOutput) -> String {
    let mut fragments: Vec<String> = vec![match output.status {
        Some(status) => status.to_string(),
        None => "Did not run.".to_string(),
    }];

    if !output.errors.is_empty() {
        fragments.push("".to_string());
        fragments.push("========== Errors ==========".to_string());
        for error in &output.errors {
            fragments.push(match error {
                CaptureError::Spawn(error) => format!("Spawn error: {}", error),
                CaptureError::Stdout(error) => format!("Error while reading stdout: {}", error),
                CaptureError::Stderr(error) => format!("Error while reading stderr: {}", error),
                CaptureError::Wait(error) => format!("Error while waiting for process: {}", error),
            });
        }
    }

    if !output.stdout.is_empty() {
        fragments.push("".to_string());
        fragments.push("========== STDOUT ==========".to_string());
        fragments.push(String::from_utf8_lossy(output.stdout.trim_ascii_end()).into_owned());
    }

    if !output.stderr.is_empty() {
        fragments.push("".to_string());
        fragments.push("========== STDERR ==========".to_string());
        fragments.push(String::from_utf8_lossy(output.stderr.trim_ascii_end()).into_owned());
    }

    fragments.join("\n")
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    let result = runner::run_forward_and_capture(&args.cmdline).await;
    let status = result.status;

    let success = match status {
        Some(status) => status.success(),
        None => false,
    };

    if !args.always_notify && success && result.is_empty() {
        return;
    } else if args.only_failures && success {
        return;
    }

    let fallback_title = match args.title {
        Some(title) => title,
        None => quote::quote_cmdline(&args.cmdline),
    };

    let title = if success {
        match args.success_title {
            Some(title) => title,
            None => format!("Success: {}", fallback_title),
        }
    } else {
        match args.failure_title {
            Some(title) => title,
            None => format!("Failure: {}", fallback_title),
        }
    };

    let priority = if success {
        args.success_priority
    } else {
        args.failure_priority
    };

    let tags = if success {
        args.success_tags
    } else {
        args.failure_tags
    };

    let body = format_post_body(result);

    let request = reqwest::Client::new()
        .post(&args.url)
        .header("title", title)
        .body(body);

    let request = match priority {
        Some(priority) => request.header("priority", priority),
        None => request,
    };

    let request = match tags {
        Some(tags) => request.header("tags", tags),
        None => request,
    };

    let request = match args.email {
        Some(email) => request.header("email", email),
        None => request,
    };

    let request = match args.icon {
        Some(icon) => request.header("icon", icon),
        None => request,
    };

    let request = if let Some(token) = args.token {
        request.bearer_auth(token)
    } else if let Some(user) = args.user {
        request.basic_auth(user, args.password)
    } else {
        request
    };

    match request.send().await.and_then(|r| r.error_for_status()) {
        Ok(_) => exit(match status {
            Some(code) => code.code().unwrap_or(255),
            None => 255,
        }),
        Err(error) => {
            eprintln!("Failed to send request to ntfy: {}", error);
            exit(37)
        }
    }
}
