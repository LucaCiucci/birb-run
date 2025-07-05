use std::{borrow::Borrow, path::Path};

use crate::{command::Command, run::execution::CommandExecutor};

pub struct NaiveExecutor;

impl CommandExecutor for NaiveExecutor {
    fn execute<C: Borrow<Command>>(&self, pwd: impl AsRef<Path>, commands: impl IntoIterator<Item = C>) {
        for command in commands {
            match command.borrow() {
                Command::Shell(cmd) => Self::exec_shell(&pwd, &cmd),
            }
        }
    }
}

impl NaiveExecutor {
    fn exec_shell(pwd: impl AsRef<Path>, cmd: &str) {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(pwd)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .stdin(std::process::Stdio::null())
            .output()
            .expect("Failed to execute command");

        if !output.status.success() {
            panic!("Command '{}' failed with exit code: {}", cmd, output.status);
        }
    }
}