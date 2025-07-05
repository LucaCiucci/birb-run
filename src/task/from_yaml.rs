use std::{path::PathBuf, str::FromStr};

use serde_json::{Number, Value as Json};
use yaml_rust::Yaml;

use crate::{task::Task};

mod command;
mod io;
mod deps;

pub fn parse_task(workdir: impl Into<PathBuf>, name: &str, value: &Yaml) -> Task {
    let value = value.as_hash().expect("Expected task value to be a hash");
    let mut task = Task::new(name);

    task.body.workdir = workdir.into();

    if let Some(value) = value.get(&Yaml::String("workdir".into())) {
        task.body.workdir = PathBuf::from(value.as_str().expect("Expected 'workdir' to be a string"));
    }

    if let Some(value) = value.get(&Yaml::String("phony".into())) {
        task.body.phony = value.as_bool().expect("Expected 'phony' to be a boolean");
    }

    if let Some(deps) = value.get(&Yaml::String("deps".into())) {
        deps::parse_deps(&mut task, deps);
    }

    if let Some(params) = value.get(&Yaml::String("params".into())) {
        io::parse_params(&mut task, params);
    }

    if let Some(steps) = value.get(&Yaml::String("steps".into())) {
        command::parse_steps(&mut task, steps);
    }

    if let Some(sources) = value.get(&Yaml::String("sources".into())) {
        io::parse_sources(&mut task, sources);
    }

    if let Some(outputs) = value.get(&Yaml::String("outputs".into())) {
        io::parse_outputs(&mut task, outputs);
    }

    task
}




fn yaml_to_json(yaml: &Yaml) -> Json {
    match yaml {
        Yaml::Null => Json::Null,
        Yaml::Boolean(b) => Json::Bool(*b),
        Yaml::Integer(i) => Json::Number(Number::from(*i)),
        Yaml::Real(s) => Json::Number(Number::from_str(s).expect("Expected a valid number")),
        Yaml::String(s) => Json::String(s.clone()),
        Yaml::Array(arr) => Json::Array(arr.iter().map(yaml_to_json).collect()),
        Yaml::Hash(hash) => {
            let obj: serde_json::Map<String, Json> = hash
                .iter()
                .map(|(k, v)| (k.as_str().unwrap().to_string(), yaml_to_json(v)))
                .collect();
            Json::Object(obj)
        },
        Yaml::Alias(_) => todo!(),
        Yaml::BadValue => panic!("Encountered a bad value in YAML"),
    }
}