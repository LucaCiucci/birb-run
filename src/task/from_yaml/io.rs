use yaml_rust::Yaml;

use crate::task::{from_yaml::yaml_to_json, ArgType, Param, Task};


pub fn parse_sources(task: &mut Task, sources: &Yaml) {
    task.body.sources = sources
        .as_vec()
        .expect("Expected 'sources' to be an array")
        .iter()
        .map(|s| s.as_str().expect("Expected source to be a string").to_string())
        .collect();
}

pub fn parse_outputs(task: &mut Task, outputs: &Yaml) {
    task.body.outputs.files = outputs
        .as_vec()
        .expect("Expected 'outputs' to be an array")
        .iter()
        .map(|s| s.as_str().expect("Expected output to be a string").to_string())
        .collect();
}

pub fn parse_params(
    task: &mut Task,
    args: &Yaml,
) {
    let args = args.as_hash().expect("Expected 'args' to be a hash");
    for (key, value) in args {
        let key = key.as_str().expect("Expected argument key to be a string").to_string();
        task.params.insert(key, parse_param(value));
    }
}

fn parse_param(value: &Yaml) -> Param {
    match value {
        Yaml::String(t) => Param {
            ty: parse_param_type_str(t),
            default: None,
        },
        Yaml::Hash(hash) => {
            let ty = hash
                .get(&Yaml::String("type".into()))
                .expect("Expected 'type' key in argument hash");
            let ty = parse_param_type(ty);
            let default = hash
                .get(&Yaml::String("default".into()))
                .map(|v| yaml_to_json(v));
            Param {
                ty,
                default,
            }
        }
        _ => panic!("Unsupported argument type"),
    }
}

fn parse_param_type(t: &Yaml) -> ArgType {
    match t {
        Yaml::String(t) => parse_param_type_str(t),
        Yaml::Array(options) => parse_param_type_select(options),
        _ => todo!(),
    }
}

fn parse_param_type_str(t: &str) -> ArgType {
    match t {
        "str" | "string" => ArgType::String,
        "number" => ArgType::Number,
        "bool" | "boolean" => ArgType::Boolean,
        "path" => ArgType::Path,
        "array" => todo!(),
        _ => panic!("Unsupported argument type: {t}"),
    }
}

fn parse_param_type_select(options: &[Yaml]) -> ArgType {
    let options = options
        .iter()
        .map(|opt| match opt {
            Yaml::String(opt) => opt.clone(),
            _ => panic!("Expected array options to be strings"),
        })
        .collect();
    ArgType::Select(options)
}
