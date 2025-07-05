use birb_run::{task::{Task, TaskInvocation}};
use linked_hash_map::LinkedHashMap;
use yaml_rust::{Yaml, YamlLoader};
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(styles = cli_styles::CLAP_STYLES)]
enum Cli {
    Run(Run),
    Clean(Run),
}

#[derive(Parser, Debug)]
struct Run {
    task: String,
}

#[derive(Parser, Debug)]
struct Clean {
    task: String,
}

fn main() {
    env_logger::init();

    //let boot = std::time::Instant::now();

    let args = Cli::parse();

    let cwd = std::env::current_dir()
        .expect("Failed to get current directory");

    let (taskfile_dir, taskfile_path) = {
        let mut dir = cwd;
        loop {
            let path = dir.join("tasks.yaml");
            if path.is_file() {
                break (dir, path);
            }
            if !dir.pop() {
                panic!("No 'tasks.yaml' found in the current directory or any parent directory");
            }
        }
    };

    let docs = YamlLoader::load_from_str(
    &std::fs::read_to_string(&taskfile_path).expect("Failed to read 'tasks.yaml' file")
    ).unwrap();

    let mut all_tasks: LinkedHashMap<String, Task> = Default::default();

    for doc in docs {
        let doc = doc
            .as_hash()
            .expect("Expected a YAML hash");

        let tasks = doc
            .get(&Yaml::String("tasks".into()))
            .expect("Expected 'tasks' key")
            .as_hash()
            .expect("Expected 'tasks' to be a hash");

        for (key, value) in tasks {
            let key = key.as_str().expect("Expected task key to be a string");
            let task = Task::from_yaml(
                &taskfile_dir,
                key,
                value,
            );
            all_tasks.insert(key.to_string(), task.clone());
        }
    }
    //let elapsed = start.elapsed();
    //println!("Elapsed time: {:?}", elapsed);

    match args {
        Cli::Run(args) => {
            let task = all_tasks
                .get(&args.task)
                .expect("Task not found");

            birb_run::run::run(&all_tasks, &TaskInvocation {
                name: task.name.clone(),
                args: Default::default(),
            });
        },
        Cli::Clean(args) => {
            let task = all_tasks
                .get(&args.task)
                .expect("Task not found");

            birb_run::run::clean(&all_tasks, &TaskInvocation {
                name: task.name.clone(),
                args: Default::default(),
            });
        }
    }

    //let boot_elapsed = boot.elapsed();
    //println!("Total: {:?}", boot_elapsed);
}

