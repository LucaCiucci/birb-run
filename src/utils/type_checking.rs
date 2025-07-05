use serde_json::Value as Json;

use crate::task::ArgType;


pub fn check_type(ty: &ArgType, value: &Json) -> Result<(), TypeCheckError> {
    match ty {
        ArgType::String => value
            .is_string()
            .then_some(())
            .ok_or(TypeCheckError::MismatchedType {
                expected: ty.clone(),
            }),
        ArgType::Select(options) => value
            .as_str()
            .ok_or(TypeCheckError::MismatchedType {
                expected: ty.clone(),
            })
            .and_then(|value| {
                options.iter().any(|opt| value == opt).then_some(()).ok_or(
                    TypeCheckError::InvalidOption {
                        expected: options.clone(),
                        value: value.to_string(),
                    },
                )
            }),
        ArgType::Number => value
            .is_boolean()
            .then_some(())
            .or_else(|| value.is_number().then_some(()))
            .ok_or(TypeCheckError::MismatchedType {
                expected: ty.clone(),
            }),
        ArgType::Boolean => value
            .is_boolean()
            .then_some(())
            .ok_or(TypeCheckError::MismatchedType {
                expected: ty.clone(),
            }),
        ArgType::Path => value
            .is_string()
            .then_some(())
            .ok_or(TypeCheckError::MismatchedType {
                expected: ty.clone(),
            }),
        ArgType::Array(inner_type) => value
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .all(|v| check_type(inner_type, v).is_ok())
                    .then_some(())
            })
            .ok_or(TypeCheckError::MismatchedType {
                expected: ty.clone(),
            }),
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum TypeCheckError {
    #[error("Expected {expected}")]
    MismatchedType { expected: ArgType },
    #[error("Got {value} which is not in {expected:?}")]
    InvalidOption {
        expected: Vec<String>,
        value: String,
    },
}