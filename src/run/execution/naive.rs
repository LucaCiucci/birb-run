use std::io::Write;
use std::{borrow::Borrow, io::BufRead, path::Path};
use std::sync::mpsc;
use std::thread;

use tempfile::NamedTempFile;

use crate::{command::Command, run::execution::CommandExecutor};

pub struct NaiveExecutor<F: FnMut(&str)> {
    pub output_handler: F,
}

impl<F: FnMut(&str)> CommandExecutor for NaiveExecutor<F> {
    fn execute<C: Borrow<Command>>(
        &mut self,
        pwd: impl AsRef<Path>,
        commands: impl IntoIterator<Item = C>,
    ) -> anyhow::Result<()> {
        for command in commands {
            match command.borrow() {
                Command::Shell(cmd) => Self::exec_shell(&pwd, &cmd, &mut self.output_handler)?,
            }
        }

        Ok(())
    }
}

impl<F: FnMut(&str)> NaiveExecutor<F> {
    fn exec_shell(pwd: impl AsRef<Path>, cmd: &str, mut output_handler: impl FnMut(&str)) -> anyhow::Result<()> {
        // try to find the shebang
        let shebang = cmd.lines().next().filter(|line| line.starts_with("#!")).map(|line| line.to_string());
        let mut script: NamedTempFile;
        let (program, args) = if let Some(shebang) = shebang {
            let interpreter = shebang.trim_start_matches("#!").trim();
            let mut args = shlex::split(interpreter).expect("Failed to parse shebang");
            assert!(!args.is_empty(), "Shebang must contain at least the interpreter");
            let program = args.remove(0);
            script = NamedTempFile::new().expect("Failed to create temporary file");
            script.write_all(cmd.as_bytes()).expect("Failed to write to temporary file");
            args.push(script.path().to_string_lossy().to_string());
            (program, args)
        } else {
            ("sh".to_string(), vec!["-c".to_string(), cmd.to_string()]) // TODO avoid useless string clone, use cow or something
        };

        let mut child = std::process::Command::new(program)
            .args(args)
            .current_dir(pwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to execute command '{}': {e}", cmd))?;

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");

        let stdout_reader = std::io::BufReader::new(stdout);
        let stderr_reader = std::io::BufReader::new(stderr);

        let (tx, rx) = mpsc::channel();

        // Spawn thread for stdout
        let tx_stdout = tx.clone();
        thread::spawn(move || {
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    tx_stdout.send(line).expect("Failed to send stdout line");
                }
            }
        });

        // Spawn thread for stderr
        thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    tx.send(line).expect("Failed to send stderr line");
                }
            }
        });

        // Process lines from both stdout and stderr
        loop {
            if let Ok(line) = rx.recv() {
                output_handler(&line);
            }

            if let Some(status) = child.try_wait().expect("Failed to query child process status") {
                if !status.success() {
                    panic!("Command '{}' failed with exit code: {}", cmd, status);
                }
                break Ok(()); // Exit the loop if the child process has finished
            }
        }
    }
}
