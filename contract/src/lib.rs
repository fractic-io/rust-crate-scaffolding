//! Shared descriptors and serialization helpers for generated repository contracts.

use fractic_server_error::{ServerError, define_internal_error, define_user_error};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};

define_user_error!(
    InvalidRepositoryInput,
    "Invalid repository input: {details}.",
    { details: &str }
);
define_user_error!(
    UnknownRepositoryOperation,
    "Repository `{repository}` has no operation `{operation}`.",
    { repository: &str, operation: &str }
);
define_internal_error!(
    RepositoryOutputSerializationFailure,
    "Failed to serialize a repository operation result."
);

// Definitions.
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Caller-facing safety class for a repository operation.
pub enum OperationClass {
    Read,
    Write,
    Destructive,
    Internal,
}

#[derive(Debug, Serialize)]
/// Describes one field in a serialized object value.
pub struct FieldDescriptor {
    pub name: &'static str,
    pub rust_type: &'static str,
    pub required: bool,
}

/// How a value is represented in the serialized operation contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueShape {
    None,
    Direct,
    Object,
}

#[derive(Debug, Serialize)]
/// Describes the serialized shape of an operation input or output.
pub struct ValueDescriptor {
    pub shape: ValueShape,
    pub rust_type: &'static str,
    pub fields: &'static [FieldDescriptor],
}

#[derive(Debug, Serialize)]
/// Describes one callable repository operation.
pub struct OperationDescriptor {
    pub name: &'static str,
    pub class: OperationClass,
    pub deprecated: bool,
    pub input: ValueDescriptor,
    pub output: ValueDescriptor,
}

#[derive(Debug, Serialize)]
/// Describes the serialized contract exposed by a repository trait.
pub struct RepositoryDescriptor {
    pub name: &'static str,
    pub repository_type: &'static str,
    pub operations: &'static [OperationDescriptor],
}

// Public interface.
// ----------------------------------------------------------------------------

/// Decodes a serialized operation input into its repository type.
pub fn decode_input<T: DeserializeOwned>(value: Value) -> Result<T, ServerError> {
    serde_json::from_value(value).map_err(|error| InvalidRepositoryInput::new(&error.to_string()))
}

/// Decodes an object after inserting its externally selected tagged variant.
pub fn decode_tagged_input<T: DeserializeOwned>(
    value: Value,
    tag: &str,
    variant: &str,
) -> Result<T, ServerError> {
    let mut object = match value {
        Value::Null => Map::new(),
        Value::Object(object) => object,
        _ => {
            return Err(InvalidRepositoryInput::new(
                "operation input must be a JSON object",
            ));
        }
    };
    object.insert(tag.to_owned(), Value::String(variant.to_owned()));
    decode_input(Value::Object(object))
}

/// Accepts only an absent or empty operation input document.
pub fn require_no_input(value: &Value) -> Result<(), ServerError> {
    match value {
        Value::Null => Ok(()),
        Value::Object(object) if object.is_empty() => Ok(()),
        _ => Err(InvalidRepositoryInput::new(
            "operation does not accept an input document",
        )),
    }
}

/// Encodes a typed repository operation result for contract consumers.
pub fn encode_output<T: Serialize>(value: T) -> Result<Value, ServerError> {
    serde_json::to_value(value)
        .map_err(|error| RepositoryOutputSerializationFailure::with_debug(&error))
}

/// Builds the standard error for an unknown operation name.
pub fn unknown_operation(repository: &str, operation: &str) -> ServerError {
    UnknownRepositoryOperation::new(repository, operation)
}
