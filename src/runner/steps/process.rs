use bytes::Bytes;
use eyre::{eyre, Result};
use futures::StreamExt;
use tokio::{io::AsyncWriteExt, sync::broadcast};
use tokio_util::io::ReaderStream;

use crate::types::PatuiStepProcess;

use super::{PatuiStepData, PatuiStepDataFlavour, PatuiStepRunnerTrait};

enum PatuiProcess {
    None,
    Std(tokio::process::Child),
    Pty(Box<dyn portable_pty::Child>),
}

pub(crate) struct PatuiStepRunnerProcess {
    step: PatuiStepProcess,

    process: PatuiProcess,

    stdin: (
        broadcast::Receiver<PatuiStepData>,
        broadcast::Sender<PatuiStepData>,
    ),
    stdout: (
        broadcast::Sender<PatuiStepData>,
        broadcast::Receiver<PatuiStepData>,
    ),
    stderr: (
        broadcast::Sender<PatuiStepData>,
        broadcast::Receiver<PatuiStepData>,
    ),
}

impl PatuiStepRunnerProcess {
    pub(crate) fn new(step: &PatuiStepProcess) -> Self {
        // TODO: Tune parameters
        let (stdin_tx, stdin_rx) = broadcast::channel(1);
        let (stdout_tx, stdout_rx) = broadcast::channel(1);
        let (stderr_tx, stderr_rx) = broadcast::channel(1);

        Self {
            step: step.clone(),

            process: PatuiProcess::None,

            stdin: (stdin_rx, stdin_tx),
            stdout: (stdout_tx, stdout_rx),
            stderr: (stderr_tx, stderr_rx),
        }
    }

    fn run_pty(&mut self, tty: (u16, u16)) -> Result<()> {
        let pty_system = portable_pty::native_pty_system();

        // Create a new pty
        let pair = pty_system
            .openpty(portable_pty::PtySize {
                rows: tty.0,
                cols: tty.1,
                pixel_width: 9,
                pixel_height: 16,
            })
            .map_err(|e| eyre!("{:?}", e))?;

        // Fork the process
        let mut cmd = portable_pty::CommandBuilder::new(&self.step.command);
        cmd.args(&self.step.args);

        self.process = PatuiProcess::Pty(
            pair.slave
                .spawn_command(cmd)
                .map_err(|e| eyre!("{:?}", e))?,
        );

        Ok(())
    }

    fn run_std(&mut self) -> Result<()> {
        let mut child = tokio::process::Command::new(&self.step.command)
            .args(&self.step.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        tracing::debug!(
            "Process started: {} {}",
            self.step.command,
            self.step
                .args
                .iter()
                .map(|x| format!("\"{}\"", x.replace("\"", "\\\"")))
                .collect::<Vec<_>>()
                .join("\" \"")
        );
        tracing::trace!("Process started: {:?}", child);

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdin_rx = self.stdin.1.subscribe();
        let stdout_tx = self.stdout.0.clone();
        let stderr_tx = self.stderr.0.clone();

        tokio::spawn(async move {
            let mut stdout = ReaderStream::new(stdout);

            while let Some(chunk) = stdout.next().await {
                match chunk {
                    Ok(chunk) => {
                        tracing::trace!("Read chunk: {:?}", chunk);
                        if let Err(e) =
                            stdout_tx.send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(chunk)))
                        {
                            panic!("Error sending chunk: {:?}", e);
                        }
                    }
                    // TODO: Handle properly
                    Err(e) => panic!("Error reading chunk: {:?}", e),
                }
            }
        });

        tokio::spawn(async move {
            let mut stderr = ReaderStream::new(stderr);

            while let Some(chunk) = stderr.next().await {
                match chunk {
                    Ok(chunk) => {
                        tracing::trace!("Read chunk: {:?}", chunk);
                        if let Err(e) =
                            stderr_tx.send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(chunk)))
                        {
                            panic!("Error sending chunk: {:?}", e);
                        }
                    }
                    // TODO: Handle properly
                    Err(e) => panic!("Error reading chunk: {:?}", e),
                }
            }
        });

        tokio::spawn(async move {
            let mut stdin = stdin;

            let mut stdin_rx = stdin_rx;

            while let Ok(chunk) = stdin_rx.recv().await {
                tracing::trace!("Received chunk to send to stdin: {:?}", chunk);
                let Ok(bytes) = chunk.data().as_bytes() else {
                    panic!("Invalid chunk: {:?}", chunk);
                };
                if let Err(e) = stdin.write(&bytes).await {
                    panic!("Error writing chunk: {:?}", e);
                }
            }
        });

        self.process = PatuiProcess::Std(child);

        Ok(())
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerProcess {
    fn setup(&mut self) -> Result<()> {
        Ok(())
    }

    fn subscribe(&self, sub: &str) -> Result<broadcast::Receiver<PatuiStepData>> {
        if self.step.tty.is_some() {
            match sub {
                _ => Err(eyre!("Invalid subscription")),
            }
        } else {
            match sub {
                "stdin" => Err(eyre!("Cannot subscribe to stdin, must use publish")),
                "stdout" => Ok(self.stdout.0.subscribe()),
                "stderr" => Ok(self.stderr.0.subscribe()),
                _ => Err(eyre!("Invalid subscription")),
            }
        }
    }

    fn publish(&self, publ: &str, data: PatuiStepData) -> Result<()> {
        assert!(data.data().is_bytes());
        match publ {
            "stdin" => {
                if let PatuiProcess::Std(_) = &self.process {
                    let stdin_tx = self.stdin.1.clone();
                    let _ = stdin_tx.send(data);
                }

                Ok(())
            }
            _ => Err(eyre!("Invalid publisher")),
        }
    }

    async fn wait(&mut self, action: &str) -> Result<PatuiStepData> {
        match action {
            "exit_code" => {
                let code: i64 = match &mut self.process {
                    PatuiProcess::Std(child) => child.wait().await?.code().unwrap_or(-1) as i64,
                    PatuiProcess::Pty(child) => child.wait()?.exit_code() as i64,
                    _ => return Err(eyre!("Process not started")),
                };

                Ok(PatuiStepData::new(PatuiStepDataFlavour::Number(code)))
            }
            _ => Err(eyre!("Invalid action")),
        }
    }

    fn check(&mut self, action: &str) -> Result<PatuiStepData> {
        match action {
            "exit_code" => {
                let status = match &mut self.process {
                    PatuiProcess::Std(child) => child
                        .try_wait()?
                        .ok_or_else(|| eyre!("Process not exited"))?
                        .code()
                        .unwrap_or(-1),
                    PatuiProcess::Pty(child) => child
                        .try_wait()?
                        .map(|x| x.exit_code() as i32)
                        .unwrap_or(-1),
                    _ => return Err(eyre!("Process not started")),
                };

                let status = format!("{}", status);

                Ok(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
                    Bytes::from(status),
                )))
            }
            _ => Err(eyre!("Invalid action")),
        }
    }

    fn run(&mut self) -> Result<bool> {
        if let Some(tty) = self.step.tty {
            self.run_pty(tty)?;
        } else {
            self.run_std()?;
        };

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::{process::Command, sync::Mutex, time::Duration};

    use assertor::*;
    use lazy_static::lazy_static;
    use tokio::time::timeout;
    use tracing_test::traced_test;

    use super::*;

    lazy_static! {
        static ref COMPILED: Mutex<bool> = Mutex::new(false);
    }

    fn compile_program() {
        let mut lock = COMPILED.lock().unwrap();

        if *lock {
            return;
        }

        *lock = true;

        tracing::trace!("Compiling json spitter program");

        let output = Command::new("cargo")
            .arg("build")
            .current_dir("test_progs/json_spitter")
            .output()
            .unwrap();

        assert!(output.status.success());
    }

    #[traced_test]
    #[tokio::test]
    async fn step_process_non_tty_without_wait() {
        compile_program();

        let mut step_runner_process = PatuiStepRunnerProcess::new(&PatuiStepProcess {
            command: "test_progs/json_spitter/target/debug/json_spitter".to_string(),
            args: vec![],
            tty: None,
            wait: false,
            input: None,
            cwd: None,
        });

        let mut stdout_rx = step_runner_process.subscribe("stdout").unwrap();

        assert_that!(step_runner_process.setup()).is_ok();
        assert_that!(step_runner_process.run()).is_ok();

        let ret = timeout(Duration::from_millis(50), stdout_rx.recv()).await;
        tracing::trace!("Received: {:?}", ret);
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_eq!(
            *ret.data(),
            PatuiStepDataFlavour::Bytes(Bytes::from(r#"{"foo":"bar"}"#))
        );

        let ret = timeout(Duration::from_millis(50), stdout_rx.recv()).await;
        tracing::trace!("Received: {:?}", ret);
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_eq!(
            *ret.data(),
            PatuiStepDataFlavour::Bytes(Bytes::from(r#"{"bar":"baz"}"#))
        );

        let ret = timeout(Duration::from_millis(50), stdout_rx.recv()).await;
        tracing::trace!("Received: {:?}", ret);
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_eq!(
            *ret.data(),
            PatuiStepDataFlavour::Bytes(Bytes::from(r#"{"baz":123}"#))
        );

        assert_that!(step_runner_process.publish(
            "stdin",
            PatuiStepData::new(PatuiStepDataFlavour::Bytes(Bytes::from(
                "{\"foo\":\"baz\"}\n"
            ))),
        ))
        .is_ok();

        let ret = timeout(Duration::from_millis(50), stdout_rx.recv()).await;
        tracing::trace!("Received: {:?}", ret);
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_eq!(
            *ret.data(),
            PatuiStepDataFlavour::Bytes(Bytes::from("{\"foo\":\"baz\"}\n"))
        );

        let ret = timeout(
            Duration::from_millis(50),
            step_runner_process.wait("exit_code"),
        )
        .await;
        tracing::trace!("Received: {:?}", ret);
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_eq!(*ret.data(), PatuiStepDataFlavour::Number(0));
    }

    #[traced_test]
    #[test]
    fn step_process_io() {
        compile_program();
    }
}
