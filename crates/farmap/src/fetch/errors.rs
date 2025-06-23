use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
#[error("Input data is not jsonl at : .path")]
pub struct InvalidJsonlError {
    pub path: String,
}

#[derive(Error, Debug, PartialEq)]
pub enum DataReadError {
    #[error("Input data is not jsonl at : .path")]
    InvalidJsonlError(#[from] InvalidJsonlError),

    #[error("The path {0} is invalid", .path)]
    InvalidDataPathError { path: String },
}
