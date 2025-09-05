use std::{borrow::Borrow, path::PathBuf};

use anyhow::anyhow;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{cli::CliRunOptions, run::{display_args, execution::{naive::NaiveExecutor, CommandExecutor}, RunExecution, RunManager, TaskExecutionContext}, task::ResolvedTaskInvocation};

pub struct DefaultRunManager<C: Borrow<CliRunOptions> + Send + Sync>(pub C); // TODO also use options while cleaning

impl<C: Borrow<CliRunOptions> + Send + Sync + Clone> RunManager for DefaultRunManager<C> {
    type RunExecution = DefaultRunExecution<C>;
    fn begin<'a>(self, invocations: impl IntoIterator<Item = &'a ResolvedTaskInvocation>) -> anyhow::Result<Self::RunExecution> {
        let bar = ProgressBar::new(invocations.into_iter().count() as u64);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.green/white}] {pos:>7}/{len:7} {msg}")?
            .progress_chars("=>-"));
        Ok(DefaultRunExecution {
            bar,
            options: self.0,
        })
    }
}

pub struct DefaultRunExecution<C: Borrow<CliRunOptions> + Send + Sync> {
    bar: ProgressBar,
    options: C,
}

impl<C: Borrow<CliRunOptions> + Send + Sync> Drop for DefaultRunExecution<C> {
    fn drop(&mut self) {
        self.bar.finish_with_message("All tasks completed");
    }
}

impl<C: Borrow<CliRunOptions> + Send + Sync + Clone> RunExecution for DefaultRunExecution<C> {
    type TaskExecutionContext<'a> = DefaultTaskExecutionContext<'a, C> where Self: 'a;
    fn enter_task<'a>(&'a self, invocation: &'a ResolvedTaskInvocation) -> anyhow::Result<Self::TaskExecutionContext<'a>> {
        self.bar.inc(1);
        let args = display_args(invocation);
        self.bar.set_message(format!("task: {} {args}", invocation.r#ref.display_relative(&std::env::current_dir().unwrap()).to_string().bold().green()));
        Ok(DefaultTaskExecutionContext {
            bar: &self.bar,
            invocation,
            cwd: std::env::current_dir().map_err(|e| anyhow!("Failed to get current directory: {e}"))?,
            options: self.options.clone(),
        })
    }
}

pub struct DefaultTaskExecutionContext<'a, C: Borrow<CliRunOptions> + Send + Sync> {
    bar: &'a ProgressBar,
    invocation: &'a ResolvedTaskInvocation,
    cwd: PathBuf,
    options: C,
}

impl<C: Borrow<CliRunOptions> + Send + Sync> TaskExecutionContext for DefaultTaskExecutionContext<'_, C> {
    fn run(&mut self) -> impl CommandExecutor {
        let args = display_args(self.invocation);
        if !self.options.borrow().compact {
            self.bar.suspend(|| {
                println!("    {} {args}\trunning...", self.invocation.r#ref.display_relative(&self.cwd).to_string().bold().green());
            });
        }
        NaiveExecutor {
            output_handler: |output| {
                // ! self.bar.suspend(|| println!("{output}"));
self.bar.suspend(|| {
    //let mut s = stderr();
    //s.queue(cursor::MoveUp(1)).unwrap();
    //s.queue(terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
    //s.flush().unwrap();
    println!("{output}");
    //s.queue(cursor::MoveToColumn(0)).unwrap();
    //writeln!(&mut s, " === OK ===").unwrap();
    //s.flush().unwrap();
});
            },
        }
    }

    fn up_to_date(&mut self) {
        if !self.options.borrow().compact {
            let args = display_args(self.invocation);
            self.bar.suspend(|| {
                println!("    {} {args}\tup-to-date.", self.invocation.r#ref.display_relative(&self.cwd).to_string().bold().cyan())
            });
        }
    }
}