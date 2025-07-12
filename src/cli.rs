use clap::Parser;
use colored::Colorize;

use crate::task::{Task, TaskInvocation, TaskRef, Taskfile, Workspace};

/// Command-line interface for the birb task runner.
///
/// This CLI provides commands to manage and execute tasks defined in YAML
/// taskfiles.
/// Tasks can have dependencies, parameters, and outputs, and the runner handles
/// dependency resolution and execution order automatically.
#[derive(Parser, Debug)]
#[clap(styles = cli_styles::CLAP_STYLES, verbatim_doc_comment)]
pub enum Cli {
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
}

/// Run a task
#[derive(Parser, Debug)]
pub struct Run {
    #[clap(default_value = "default")]
    task: String,
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

pub fn main(args: &Cli) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().expect("Failed to get current directory");

    let (workspace, tasks_id) = Workspace::from_main(cwd)?;
    let tasks = workspace.get(&tasks_id).expect("Failed to get taskfile from workspace");

    match args {
        Cli::List(args) => list(&tasks, args)?,
        Cli::Run(args) => tasks.invoke(&workspace, &TaskInvocation::no_args(TaskRef::parse(&args.task)))?,
        Cli::Clean(args) => tasks.clean(&workspace, &TaskInvocation::no_args(TaskRef::parse(&args.task)), true)?,
        Cli::CleanOnly(args) => tasks.clean(&workspace, &TaskInvocation::no_args(TaskRef::parse(&args.task)), false)?,
    };

    Ok(())
}

fn list(tasks: &Taskfile, args: &List) -> anyhow::Result<()> {
    for task in tasks.tasks.values() {
        let help = || task_short(task)
            .map(|s| format!("# {}", termimad::inline(&s)).green())
            .unwrap_or_else(|| "".to_string().normal());

        if args.names_only {
            println!("{}", task.name.cyan().bold());
            continue;
        } else if args.short {
            let args = task.params
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
            for (name, param) in &task.params {
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
