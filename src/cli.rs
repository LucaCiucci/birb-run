use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use colored::Colorize;
use log::LevelFilter;

use crate::{cli::threads_config::ThreadsConfig, task::{Task, TaskInvocation, TaskRef, Taskfile, Workspace}};

pub mod threads_config;
pub mod value_parser;

/// Command-line interface for the birb task runner.
///
/// This CLI provides commands to manage and execute tasks defined in YAML
/// taskfiles.
/// Tasks can have dependencies, parameters, and outputs, and the runner handles
/// dependency resolution and execution order automatically.
#[derive(Parser, Debug)]
#[clap(styles = cli_styles::CLAP_STYLES, verbatim_doc_comment)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,

    /// Path to the taskfile or search path.
    /// If not provided, the runner will look for a
    /// `Taskfile.yaml` in the current directory.
    #[clap(short = 'f', long, value_name = "PATH")]
    pub taskfile: Option<PathBuf>,

    #[clap(short = 'v', long)]
    pub log_level: Option<LogLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(ValueEnum)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Parser, Debug)]
pub enum Command {
    List(List),
    Run(Run),
    Clean(Clean),
    CleanOnly(CleanOnly),
}

/// List all tasks
#[derive(Parser, Debug)]
pub struct List {
    /// List tasks in short format
    #[clap(short, long)]
    short: bool,

    /// Only show task names
    #[clap(short, long)]
    names_only: bool,

    /// Show full description for each task
    #[clap(short, long)]
    description: bool,

    /// Output format
    #[clap(short, long, value_enum)]
    format: Option<OutputFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(ValueEnum)]
pub enum OutputFormat {
    Json,
}

/// Run a task
#[derive(Parser, Debug)]
pub struct Run {
    #[clap(default_value = "default")]
    task: String,

    #[clap(flatten)]
    options: CliRunOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Parser)]
pub struct CliRunOptions {
    /// Less verbose, only show progress and not the tasks name and status
    #[clap(long)]
    pub compact: bool,

    /// Number of threads to use for parallel execution.
    ///
    /// Using this option enable parallel execution mode using the specified number of threads.
    #[clap(short = 'j', long)]
    pub threads: Option<ThreadsConfig>,
}

/// Recursively clean a task
#[derive(Parser, Debug)]
pub struct Clean {
    #[clap(default_value = "default")]
    task: String,
}

/// Clean a single task (non recursive)
#[derive(Parser, Debug)]
pub struct CleanOnly {
    task: String,
}

pub fn main(args: &Cli, init_env_logger: bool) -> anyhow::Result<()> {
    if init_env_logger {
        let mut b = env_logger::builder();
        if let Some(level) = args.log_level {
            let filter = match level {
                LogLevel::Off => LevelFilter::Off,
                LogLevel::Error => LevelFilter::Error,
                LogLevel::Warn => LevelFilter::Warn,
                LogLevel::Info => LevelFilter::Info,
                LogLevel::Debug => LevelFilter::Debug,
                LogLevel::Trace => LevelFilter::Trace,
            };
            b.filter_level(filter);
        }
        b.init();
    }

    let cwd: PathBuf;
    let path = if let Some(taskfile) = &args.taskfile {
        taskfile
    } else {
        cwd = std::env::current_dir()?;
        &cwd
    };

    let (workspace, tasks_id) = Workspace::from_main(path)?;
    let tasks = workspace.get(&tasks_id).expect("Failed to get taskfile from workspace");

    match &args.command {
        Command::List(args) => list(&tasks, args)?,
        Command::Run(args) => tasks.invoke(&workspace, &TaskInvocation::no_args(TaskRef::parse(&args.task)), &args.options)?,
        Command::Clean(args) => tasks.clean(&workspace, &TaskInvocation::no_args(TaskRef::parse(&args.task)), true)?,
        Command::CleanOnly(args) => tasks.clean(&workspace, &TaskInvocation::no_args(TaskRef::parse(&args.task)), false)?,
    };

    Ok(())
}

fn list(tasks: &Taskfile, args: &List) -> anyhow::Result<()> {
    if let Some(format) = args.format {
        if format != OutputFormat::Json {
            todo!("")
        }

        #[derive(serde::Serialize)]
        struct TaskEntry {
            name: String,
            short: Option<String>,
            description: Option<String>,
            // TODO params: Vec<(String, String)>,
        }

        let entries = tasks.tasks.values().map(|task| {
            TaskEntry {
                name: task.name.clone(),
                short: task_short(task),
                description: task.description.clone(),
            }
        }).collect::<Vec<_>>();

        let json = serde_json::to_string(&entries)?;
        println!("{}", json);

        return Ok(())
    }

    for task in tasks.tasks.values() {
        let help = || task_short(task)
            .map(|s| format!("# {}", termimad::inline(&s)).green())
            .unwrap_or_else(|| "".to_string().normal());

        if args.names_only {
            println!("{}", task.name.cyan().bold());
            continue;
        } else if args.short {
            let args = task.params
                .0
                .iter()
                .map(|(name, _)| format!("<{name}>"))
                .collect::<Vec<_>>()
                .join(" ");
            let line = format!("{} {}", task.name.cyan().bold(), args.cyan());
            println!("{line:<50} {}", help());
        } else {
            println!("{:<20} {}", task.name.cyan().bold(), if !args.description { help() } else { "".normal() });
            if args.description {
                if let Some(desc) = &task.description {
                    let desc = termimad::text(desc).to_string();
                    for line in desc.lines() {
                        let line = format!("  # {}", termimad::inline(line));
                        println!("{}", line.green());
                    }
                }
            }
            for (name, param) in &task.params.0 {
                let ty = param.ty.to_string();
                let default = param.default.as_ref().map_or("".to_string(), |d| format!(" (default: {d})"));
                println!("  {}: {}{}", name.cyan(), ty, default);
            }
        }
    }

    Ok(())
}

fn task_short(task: &Task) -> Option<String> {
    let desc: &str = task.description.as_ref()?;

    // take until empty line or end of string
    let short = desc
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join(" ");

    Some(short)
}
