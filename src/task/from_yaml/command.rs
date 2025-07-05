use yaml_rust::Yaml;

use crate::{command::Command, task::Task};



pub fn parse_steps(task: &mut Task, steps: &Yaml) {
    task.body.steps = steps
        .as_vec()
        .expect("Expected 'steps' to be an array")
        .iter()
        .map(|step| match step {
            Yaml::String(cmd) => Command::Shell(cmd.clone()),
            Yaml::Hash(cmd) => {
                if let Some(run) = cmd.get(&Yaml::String("run".into())) {
                    if let Yaml::String(run_cmd) = run {
                        return Command::Shell(run_cmd.clone());
                    } else {
                        panic!("Expected 'run' to be a string");
                    }
                } else {
                    todo!();
                }
            }
            _ => todo!(),
        })
        .collect();
}