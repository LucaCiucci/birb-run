use std::{borrow::Borrow, path::PathBuf, sync::Mutex};

use anyhow::anyhow;
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::{cli::CliRunOptions, run::{display_args, execution::{naive::NaiveExecutor, CommandExecutor}, RunExecution, RunManager, TaskExecutionContext}, task::ResolvedTaskInvocation};

pub struct ParallelRunManager<C: Borrow<CliRunOptions> + Send + Sync>(pub C); // TODO also use options while cleaning

impl<C: Borrow<CliRunOptions> + Send + Sync + Clone> RunManager for ParallelRunManager<C> {
    type RunExecution = ParallelRunExecution<C>;
    fn begin<'a>(self, invocations: impl IntoIterator<Item = &'a ResolvedTaskInvocation>) -> anyhow::Result<Self::RunExecution> {
        let m = MultiProgress::new();
        //let t = m.add(ProgressBar::new_spinner());
        //t.finish_with_message("^^^\n".repeat(3).red().to_string());
        let bar = m.add(ProgressBar::new(invocations.into_iter().count() as u64));
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.green/white}] {pos:>7}/{len:7} {msg}")?
            .progress_chars("=>-"));
        Ok(ParallelRunExecution {
            bar,
            _m: m,
            counter: Mutex::new(0),
            options: self.0,
            last_was: Mutex::new(std::usize::MAX),
        })
    }
}

const COLOR_RING: &[colored::Color] = &[
    colored::Color::Black,
    colored::Color::Red,
    colored::Color::Green,
    colored::Color::Yellow,
    colored::Color::Blue,
    colored::Color::Magenta,
    colored::Color::Cyan,
    colored::Color::White,
];

pub struct ParallelRunExecution<C: Borrow<CliRunOptions> + Send + Sync> {
    bar: ProgressBar,
    _m: MultiProgress,
    counter: Mutex<usize>,
    options: C,
    last_was: Mutex<usize>
}

impl<C: Borrow<CliRunOptions> + Send + Sync> Drop for ParallelRunExecution<C> {
    fn drop(&mut self) {
        self.bar.finish_with_message("All tasks completed");
    }
}

impl<C: Borrow<CliRunOptions> + Send + Sync + Clone> RunExecution for ParallelRunExecution<C> {
    type TaskExecutionContext<'a> = ParallelTaskExecutionContext<'a, C> where Self: 'a;
    fn enter_task<'a>(&'a self, invocation: &'a ResolvedTaskInvocation) -> anyhow::Result<Self::TaskExecutionContext<'a>> {
        self.bar.inc(1);
        let args = display_args(invocation);
        self.bar.set_message(format!("task: {} {args}", invocation.r#ref.display_relative(&std::env::current_dir().unwrap()).to_string().bold().green()));
        let t = self._m.insert_before(&self.bar, ProgressBar::new_spinner());
        t.set_style(
            ProgressStyle::with_template("  {spinner:.green.bold} {msg}")
                .unwrap()
        );
        let idx = {
            let mut counter = self.counter.lock().unwrap();
            let idx = *counter;
            *counter += 1;
            idx
        };
        let color = COLOR_RING[idx % COLOR_RING.len()];
        let display_id = format!("#{idx:<5}").color(color);
        t.set_message(format!("{} {display_id} {} {}", "task".cyan().bold(), invocation.r#ref.display_relative(&std::env::current_dir().unwrap()).to_string().bold().green(), display_args(invocation)));
        Ok(ParallelTaskExecutionContext {
            bar: &self.bar,
            invocation,
            cwd: std::env::current_dir().map_err(|e| anyhow!("Failed to get current directory: {e}"))?,
            options: self.options.clone(),
            t,
            idx,
            last_was: &self.last_was,
        })
    }
}

pub struct ParallelTaskExecutionContext<'a, C: Borrow<CliRunOptions> + Send + Sync> {
    bar: &'a ProgressBar,
    last_was: &'a Mutex<usize>,
    invocation: &'a ResolvedTaskInvocation,
    cwd: PathBuf,
    options: C,
    t: ProgressBar,
    idx: usize,
}

impl<C: Borrow<CliRunOptions> + Send + Sync> Drop for ParallelTaskExecutionContext<'_, C> {
    fn drop(&mut self) {
        // HACK without this, in the end the last task spinner remains, I don't know why
        self.t.finish_and_clear();
    }
}

impl<C: Borrow<CliRunOptions> + Send + Sync> TaskExecutionContext for ParallelTaskExecutionContext<'_, C> {
    fn run(&mut self) -> impl CommandExecutor {
        let args = display_args(self.invocation);
        if !self.options.borrow().compact {
            self.bar.suspend(|| {
                println!("    {} {args}\trunning... #{}", self.invocation.r#ref.display_relative(&self.cwd).to_string().bold().green(), self.idx);
            });
        }
        NaiveExecutor {
            output_handler: |output| {
                self.t.inc(1);

                let mut first_output_part: &str = output;
                let mut second_output_part: &str = "";

                // try to find a set title escape sequence, take it and remove it from output
                // this is a hack, but I don't know how to do it better
                let set_title_prefix = "\u{1b}]0;";
                if let Some(start) = output.find(set_title_prefix) {
                    if let Some(end) = output[start + set_title_prefix.len()..].find('\u{7}') {
                        let title = &output[start + set_title_prefix.len()..start + set_title_prefix.len() + end];
                        first_output_part = &output[..start];
                        second_output_part = &output[start + set_title_prefix.len() + end + 1..];
                        self.t.set_message(format!("{} TITLE {title}", "task".cyan().bold()));
                    }
                }

                // ! self.bar.suspend(|| println!("{output}"));
                self.bar.suspend(|| {
                    let color = COLOR_RING[self.idx % COLOR_RING.len()];
                    let mut last = self.last_was.lock().unwrap();
                    let prefix = if *last == self.idx {
                        format!("       | ")
                    } else {
                        format!("#{:<5} | ", self.idx)
                    }.color(color).dimmed();
                    *last = self.idx;
                    println!("{prefix}{first_output_part}{second_output_part}");
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