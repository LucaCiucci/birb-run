use yaml_rust::Yaml;

use crate::task::{Dep, Task, TaskInvocation, from_yaml::yaml_to_json};

pub fn parse_deps(task: &mut Task, deps: &Yaml) {
    match deps {
        Yaml::Array(deps) => {
            for value in deps {
                let dep = match value {
                    Yaml::String(name) => Dep {
                        invocation: TaskInvocation {
                            name: name.clone(),
                            args: Default::default(),
                        },
                    },
                    Yaml::Hash(value) => {
                        let Some(name) = value.get(&Yaml::String("task".into())) else {
                            todo!("some other options, e.g. 'target'")
                        };
                        let mut dep = Dep {
                            invocation: TaskInvocation {
                                name: name
                                    .as_str()
                                    .expect("Expected dependency key to be a string")
                                    .to_string(),
                                args: Default::default(),
                            },
                        };

                        if let Some(args) = value.get(&Yaml::String("with".into())) {
                            let Yaml::Hash(args) = args else {
                                panic!("expected arguments to be a hash");
                            };
                            for (arg_key, arg_value) in args {
                                dep.invocation.args.insert(
                                    arg_key
                                        .as_str()
                                        .expect("Expected argument key to be a string")
                                        .to_string(),
                                    yaml_to_json(arg_value),
                                );
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
}
