use yaml_rust::Yaml;

use crate::task::{from_yaml::{yaml_to_json, YamlToJsonError}, ArgType, Param, Task};

pub fn parse_sources(task: &mut Task, sources: &Yaml) {
    task.body.sources = sources
        .as_vec()
        .expect("Expected 'sources' to be an array")
        .iter()
        .map(|s| {
            s.as_str()
                .expect("Expected source to be a string")
                .to_string()
        })
        .collect();
}

pub fn parse_outputs(task: &mut Task, outputs: &Yaml) {
    task.body.outputs.files = outputs
        .as_vec()
        .expect("Expected 'outputs' to be an array")
        .iter()
        .map(|s| {
            s.as_str()
                .expect("Expected output to be a string")
                .to_string()
        })
        .collect();
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
        task.params.insert(key, value);
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
