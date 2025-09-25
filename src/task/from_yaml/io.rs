use linked_hash_map::LinkedHashMap;
use yaml_rust::Yaml;
use serde_json::Value as Json;

use crate::task::{from_yaml::{yaml_to_json, YamlToJsonError}, ArgType, OutputPath, Param, Task};

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum InvalidEnv {
    #[error("Invalid environment, expected a map")]
    NotAHash,
    #[error("Invalid environment key, expected a string but got: {0:?}")]
    InvalidKey(Yaml),
    #[error("Invalid environment value, could not convert to json value: {0:?}")]
    InvalidValue(YamlToJsonError),
}

pub fn parse_env(value: &Yaml) -> Result<LinkedHashMap<String, Json>, InvalidEnv> {
    let hash = value
        .as_hash()
        .ok_or(InvalidEnv::NotAHash)?;
    let mut env = LinkedHashMap::new();
    for (key, val) in hash {
        let key = key
            .as_str()
            .ok_or_else(|| InvalidEnv::InvalidKey(key.clone()))?
            .to_string();
        let val = yaml_to_json(val)
            .map_err(InvalidEnv::InvalidValue)?;
        env.insert(key, val);
    }
    Ok(env)
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum InvalidSources {
    #[error("Invalid sources, expected an array")]
    NotAnArray,
    #[error("Invalid source at index {0}, expected a string but got: {1:?}")]
    NotAString(usize, Yaml),
}

pub fn parse_sources(task: &mut Task, sources: &Yaml) -> Result<(), InvalidSources> {
    task.body.sources = sources
        .as_vec()
        .ok_or(InvalidSources::NotAnArray)?
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.as_str()
                .ok_or_else(|| InvalidSources::NotAString(i, s.clone()))
                .map(|s| s.to_string())
        })
        .collect::<Result<_, _>>()?;
    Ok(())
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum InvalidOutputs {
    #[error("Invalid outputs, expected an array")]
    NotAnArray,
    #[error("Invalid output at index {0}, expected a string but got: {1:?}")]
    NotAString(usize, Yaml),
}


pub fn parse_outputs(task: &mut Task, outputs: &Yaml) -> Result<(), InvalidOutputs> {
    task.body.outputs.paths = outputs
        .as_vec()
        .ok_or(InvalidOutputs::NotAnArray)?
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.as_str()
                .ok_or_else(|| InvalidOutputs::NotAString(i, s.clone()))
                .map(|s| if s.ends_with("/") {
                    OutputPath::Directory(s.to_string())
                } else {
                    OutputPath::File(s.to_string())
                })
        })
        .collect::<Result<_, _>>()?;
    Ok(())
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum ParamParsingError {
    #[error("Invalid parameters, expected a map")]
    NotAHash,
    #[error("Invalid parameter key, expected a string but got: {0:?}")]
    InvalidKey(Yaml),
    #[error("Invalid parameter object for `{0}`: {1}")]
    InvalidParameter(String, InvalidParam),
}

pub fn parse_params(task: &mut Task, args: &Yaml) -> Result<(), ParamParsingError> {
    let args = args
        .as_hash()
        .ok_or(ParamParsingError::NotAHash)?;
    for (key, value) in args {
        let key = key
            .as_str()
            .ok_or_else(|| ParamParsingError::InvalidKey(key.clone()))?
            .to_string();
        let value = parse_param(value)
            .map_err(|e| ParamParsingError::InvalidParameter(key.clone(), e))?;
        task.params.0.insert(key, value);
    }
    Ok(())
}

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum InvalidParam {
    #[error("missing parameter type")]
    MissingType,
    #[error("invalid parameter type: {0}")]
    InvalidType(#[from] ParamTypeError),
    #[error("error converting default value for parameter `{0}`: {1}")]
    DefaultValueConversion(String, YamlToJsonError),
}

fn parse_param(value: &Yaml) -> Result<Param, InvalidParam> {
    let value = match value {
        Yaml::String(t) => Param {
            ty: parse_param_type_str(t)?,
            default: None,
        },
        Yaml::Hash(hash) => {
            let ty = hash
                .get(&Yaml::String("type".into()))
                .ok_or(InvalidParam::MissingType)?;
            let ty = parse_param_type(ty)?;
            let default = hash
                .get(&Yaml::String("default".into()))
                .map(|v| yaml_to_json(v)
                    .map_err(|e| InvalidParam::DefaultValueConversion(ty.to_string(), e)))
                .transpose()?;
            Param { ty, default }
        }
        _ => panic!("Unsupported argument type"),
    };

    Ok(value)
}

#[derive(Debug, Clone)]
#[derive(thiserror::Error)]
pub enum ParamTypeError {
    #[error("Unknown type `{0}`")]
    UnknownType(String),
    #[error("Expected array options to be strings")]
    OptionsNotStrings,
}

fn parse_param_type(t: &Yaml) -> Result<ArgType, ParamTypeError> {
    match t {
        Yaml::String(t) => parse_param_type_str(t),
        Yaml::Array(options) => parse_param_type_select(options),
        _ => todo!(),
    }
}

fn parse_param_type_str(t: &str) -> Result<ArgType, ParamTypeError> {
    match t {
        "str" | "string" => Ok(ArgType::String),
        "number" => Ok(ArgType::Number),
        "bool" | "boolean" => Ok(ArgType::Boolean),
        "path" => Ok(ArgType::Path),
        "array" => todo!(),
        _ => Err(ParamTypeError::UnknownType(t.to_string())),
    }
}

fn parse_param_type_select(options: &[Yaml]) -> Result<ArgType, ParamTypeError> {
    let options = options
        .iter()
        .map(|opt| match opt {
            Yaml::String(opt) => Ok(opt.clone()),
            _ => Err(ParamTypeError::OptionsNotStrings),
        })
        .collect::<Result<_, _>>()?;
    Ok(ArgType::Select(options))
}
