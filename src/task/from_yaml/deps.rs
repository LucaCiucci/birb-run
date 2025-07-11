use yaml_rust::Yaml;

use crate::task::{from_yaml::{yaml_to_json, YamlToJsonError}, Dep, Task, TaskInvocation, TaskRef};

#[derive(Debug)]
#[derive(thiserror::Error)]
pub enum DepParsingError {
    #[error("Invalid arguments, expected a map")]
    ArgumentsNotAHash,
    #[error("Invalid argument key, expected a string but got: {0:?}")]
    InvalidArgumentKey(Yaml),
    #[error("argument conversion error for `{0}`: {1}")]
    ArgumentConversionError(String, YamlToJsonError),
}

pub fn parse_deps(task: &mut Task, deps: &Yaml) -> Result<(), DepParsingError> {
    match deps {
        Yaml::Array(deps) => {
            for value in deps {
                let dep = match value {
                    Yaml::String(name) => Dep {
                        invocation: TaskInvocation {
                            r#ref: TaskRef::parse(name),
                            args: Default::default(),
                        },
                    },
                    Yaml::Hash(value) => {
                        let Some(name) = value.get(&Yaml::String("task".into())) else {
                            todo!("some other options, e.g. 'target'")
                        };
                        let mut dep = Dep {
                            invocation: TaskInvocation {
                                r#ref: TaskRef::parse(name.as_str().expect("Expected dependency key to be a string")),
                                args: Default::default(),
                            },
                        };

                        if let Some(args) = value.get(&Yaml::String("with".into())) {
                            let Yaml::Hash(args) = args else {
                                return Err(DepParsingError::ArgumentsNotAHash);
                            };
                            for (arg_key, arg_value) in args {
                                let arg_key = arg_key
                                    .as_str()
                                    .ok_or_else(|| DepParsingError::InvalidArgumentKey(arg_key.clone()))?
                                    .to_string();
                                let value = yaml_to_json(arg_value)
                                    .map_err(|e| DepParsingError::ArgumentConversionError(arg_key.clone(), e))?;
                                dep.invocation.args.insert(arg_key, value);
                            }
                        }

                        dep
                    }
                    _ => panic!(),
                };
                task.body.deps.0.push(dep)
            }
        }
        _ => panic!("Expected array"),
    }

    Ok(())
}
