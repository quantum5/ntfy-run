use std::process::{ExitStatus, Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::select;

pub enum CaptureError {
    Spawn(std::io::Error),
    Stdout(std::io::Error),
    Stderr(std::io::Error),
    Wait(std::io::Error),
}

pub struct CapturedOutput {
    pub status: Option<ExitStatus>,
    pub stdout: String,
    pub stderr: String,
    pub errors: Vec<CaptureError>,
}

impl CapturedOutput {
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty() && self.stdout.is_empty() && self.stderr.is_empty()
    }
}

pub async fn run_forward_and_capture(cmdline: &Vec<String>) -> CapturedOutput {
    let command = cmdline.first().unwrap();
    let ref args = cmdline[1..];

    let mut child = match Command::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(process) => process,
        Err(error) => {
            return CapturedOutput {
                status: None,
                stdout: "".to_string(),
                stderr: "".to_string(),
                errors: vec![CaptureError::Spawn(error)],
            }
        }
    };

    let mut stdout_stream = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut stderr_stream = BufReader::new(child.stderr.take().unwrap()).lines();

    let mut stdout_buffer = Vec::new();
    let mut stderr_buffer = Vec::new();
    let mut errors = Vec::new();

    let status = loop {
        select! {
            line = stdout_stream.next_line() => match line {
                Ok(Some(line)) => {
                    println!("{}", line);
                    stdout_buffer.push(line);
                },
                Ok(None) => (),
                Err(error) => errors.push(CaptureError::Stdout(error)),
            },
            line = stderr_stream.next_line() => match line {
                Ok(Some(line)) => {
                    eprintln!("{}", line);
                    stderr_buffer.push(line);
                },
                Ok(None) => (),
                Err(error) => errors.push(CaptureError::Stderr(error)),
            },
            status = child.wait() => match status {
                Ok(status) => break status,
                Err(error) => errors.push(CaptureError::Wait(error)),
            }
        }
    };

    CapturedOutput {
        status: Some(status),
        stdout: stdout_buffer.join("\n").to_string(),
        stderr: stderr_buffer.join("\n").to_string(),
        errors,
    }
}
