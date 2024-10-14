use crate::tap_stream::{ReadOrWrite, TapStream};
use std::process::{ExitStatus, Stdio};
use tokio::process::Command;
use tokio::{io, select};

pub enum CaptureError {
    Spawn(io::Error),
    Stdout(io::Error),
    Stderr(io::Error),
    Wait(io::Error),
}

pub struct CapturedOutput {
    pub status: Option<ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
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
                stdout: vec![],
                stderr: vec![],
                errors: vec![CaptureError::Spawn(error)],
            }
        }
    };

    let mut stdout_tap = TapStream::new(child.stdout.take().unwrap(), io::stdout());
    let mut stderr_tap = TapStream::new(child.stderr.take().unwrap(), io::stderr());

    let mut stdout = vec![];
    let mut stderr = vec![];
    let mut errors = Vec::new();

    let mut stdout_eof = false;
    let mut stderr_eof = false;
    let mut maybe_status: Option<ExitStatus> = None;

    let status = loop {
        select! {
            result = stdout_tap.step(), if !stdout_eof => match result {
                Ok(ReadOrWrite::Read(bytes)) => stdout.extend_from_slice(bytes),
                Ok(ReadOrWrite::Written) => (),
                Ok(ReadOrWrite::EOF) => stdout_eof = true,
                Err(error) => errors.push(CaptureError::Stdout(error)),
            },
            result = stderr_tap.step(), if !stderr_eof => match result {
                Ok(ReadOrWrite::Read(bytes)) => stderr.extend_from_slice(bytes),
                Ok(ReadOrWrite::Written) => (),
                Ok(ReadOrWrite::EOF) => stderr_eof = true,
                Err(error) => errors.push(CaptureError::Stderr(error)),
            },
            status = child.wait(), if maybe_status.is_none() => match status {
                Ok(status) => maybe_status = Some(status),
                Err(error) => errors.push(CaptureError::Wait(error)),
            },
            else => break maybe_status.unwrap(),
        }
    };

    CapturedOutput {
        status: Some(status),
        stdout,
        stderr,
        errors,
    }
}
