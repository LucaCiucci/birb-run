use std::{path::PathBuf, str::FromStr};

use serde_json::{Number, Value as Json};
use yaml_rust::Yaml;

use crate::task::Task;

mod command;
mod deps;
mod io;

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum InvalidTaskObject {
    #[error("Invalid task, expected a map")]
    InvalidTaskType,
    #[error("Invalid task key, expected a string")]
    InvalidDescriptionType,
    #[error("Invalid workdir, expected a string")]
    InvalidWorkdirType,
    #[error("Invalid phony, expected a boolean")]
    InvalidPhonyType,
    #[error("Invalid dependencies: {0}")]
    InvalidDependencies(#[from] deps::DepParsingError),
    #[error("Invalid parameters: {0}")]
    InvalidParams(#[from] io::ParamParsingError),
}

pub fn parse_task(workdir: impl Into<PathBuf>, name: &str, value: &Yaml) -> Result<Task, InvalidTaskObject> {
    let value = value
        .as_hash()
        .ok_or(InvalidTaskObject::InvalidTaskType)?;
    let mut task = Task::new(name);

    task.body.workdir = workdir.into();

    if let Some(description) = value.get(&Yaml::String("description".into())) {
        let description = description
            .as_str()
            .ok_or(InvalidTaskObject::InvalidDescriptionType)?
            .to_string();
        task.description = Some(description);
    }

    if let Some(value) = value.get(&Yaml::String("workdir".into())) {
        task.body.workdir = PathBuf::from(
            value
                .as_str()
                .ok_or(InvalidTaskObject::InvalidWorkdirType)?,
        );
    }

    if let Some(value) = value.get(&Yaml::String("phony".into())) {
        task.body.phony = value
            .as_bool()
            .ok_or(InvalidTaskObject::InvalidPhonyType)?;
    }

    if let Some(deps) = value.get(&Yaml::String("deps".into())) {
        deps::parse_deps(&mut task, deps)?;
    }

    if let Some(params) = value.get(&Yaml::String("params".into())) {
        io::parse_params(&mut task, params)?;
    }

    // TODO error handling from now on...

    if let Some(steps) = value.get(&Yaml::String("steps".into())) {
        command::parse_steps(&mut task, steps);
    }

    if let Some(clean) = value.get(&Yaml::String("clean".into())) {
        command::parse_clean(&mut task, clean);
    }

    if let Some(sources) = value.get(&Yaml::String("sources".into())) {
        io::parse_sources(&mut task, sources);
    }

    if let Some(outputs) = value.get(&Yaml::String("outputs".into())) {
        io::parse_outputs(&mut task, outputs);
    }

    Ok(task)
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum YamlToJsonError {
    #[error("Invalid number: {0}")]
    InvalidNumber(serde_json::Error),
    #[error("Invalid key, not a string: {0:?}")]
    InvalidKey(Yaml),

}

fn yaml_to_json(yaml: &Yaml) -> Result<Json, YamlToJsonError> {
    let r = match yaml {
        Yaml::Null => Json::Null,
        Yaml::Boolean(b) => Json::Bool(*b),
        Yaml::Integer(i) => Json::Number(Number::from(*i)),
        Yaml::Real(s) => Json::Number(Number::from_str(s).map_err(YamlToJsonError::InvalidNumber)?),
        Yaml::String(s) => Json::String(s.clone()),
        Yaml::Array(arr) => Json::Array(arr.iter().map(yaml_to_json).collect::<Result<_, _>>()?),
        Yaml::Hash(hash) => {
            let obj: serde_json::Map<String, Json> = hash
                .iter()
                .map(|(k, v)| Ok((
                    k.as_str().ok_or_else(|| YamlToJsonError::InvalidKey(k.clone()))?.to_string(),
                    yaml_to_json(v)?,
                )))
                .collect::<Result<_, _>>()?;
            Json::Object(obj)
        }
        Yaml::Alias(_) => todo!(),
        Yaml::BadValue => panic!("Encountered a bad value in YAML"),
    };

    Ok(r)
}
