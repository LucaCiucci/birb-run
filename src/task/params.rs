use std::fmt::Display;

use serde_json::Value as Json;

#[derive(Debug, Clone)]
pub enum ArgType {
    String,
    Select(Vec<String>),
    Number,
    Boolean,
    Path,
    Array(Box<ArgType>),
}

impl Display for ArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgType::String => write!(f, "string"),
            ArgType::Select(options) => write!(f, "opt({})", options.join(", ")),
            ArgType::Number => write!(f, "number"),
            ArgType::Boolean => write!(f, "boolean"),
            ArgType::Path => write!(f, "path"),
            ArgType::Array(inner_type) => write!(f, "array<{}>", inner_type),
        }
    }
}

impl ArgType {
    pub fn validate(&self, value: &Json) -> bool {
        match self {
            ArgType::String => value.is_string(),
            ArgType::Select(options) => {
                if let Some(s) = value.as_str() {
                    options.contains(&s.to_string())
                } else {
                    false
                }
            }
            ArgType::Number => value.is_number(),
            ArgType::Boolean => value.is_boolean(),
            ArgType::Path => value.is_string(), // Assuming path is a string
            ArgType::Array(inner_type) => {
                if let Some(arr) = value.as_array() {
                    arr.iter().all(|v| inner_type.validate(v))
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub ty: ArgType,
    pub default: Option<Json>,
}

impl Param {
    pub fn validate_default(&self) -> bool {
        if let Some(default) = &self.default {
            self.ty.validate(default)
        } else {
            true // If no default is provided, we consider it valid
        }
    }
}