use yaml_rust::Yaml;

use crate::{command::Command, task::Task};

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum StepsParseError {
    #[error("Invalid steps, expected a string or an array of strings")]
    NotStringOrArrayOfStrings,
    #[error("Invalid step at index {0}: expected a string or `run: <cmd>`, but got: {1:?}")]
    InvalidStep(usize, Yaml),
    #[error("Invalid step at index {0}: `run` expects a string, but got: {1:?}")]
    RunEntryNotAString(usize, Yaml),
}

pub fn parse_steps(task: &mut Task, steps: &Yaml) -> Result<(), StepsParseError> {
    task.body.steps = parse_steps_impl(steps)?;
    Ok(())
}

pub fn parse_clean(task: &mut Task, steps: &Yaml) -> Result<(), StepsParseError> {
    task.body.clean = Some(parse_steps_impl(steps)?);
    Ok(())
}

fn parse_steps_impl(steps: &Yaml) -> Result<Vec<Command>, StepsParseError> {
    match steps {
        Yaml::String(cmd) => Ok(vec![Command::Shell(cmd.clone())]),
        Yaml::Array(steps) => steps
            .iter()
            .enumerate()
            .map(|(i, step)| match step {
                Yaml::String(cmd) => Ok(Command::Shell(cmd.clone())),
                Yaml::Hash(cmd) => {
                    if let Some(run) = cmd.get(&Yaml::String("run".into())) {
                        if let Yaml::String(run_cmd) = run {
                            return Ok(Command::Shell(run_cmd.clone()));
                        } else {
                            return Err(StepsParseError::RunEntryNotAString(i, run.clone()));
                        }
                    } else {
                        return Err(StepsParseError::InvalidStep(i, step.clone()));
                    }
                }
                _ => todo!(),
            })
            .collect(),
        _ => return Err(StepsParseError::NotStringOrArrayOfStrings),
    }
}
