//! Import data from local files with spam label data.
//!
//! The data can be added to a [UserCollection](crate::UserCollection).
use super::DataReadError;
use crate::fetch::InvalidJsonlError;
use crate::UnprocessedUserLine;
use itertools::Itertools;
use serde_jsonlines::json_lines;
use std::fs::read_dir;

pub fn import_data_from_file_with_collected_res(
    path: &str,
) -> Result<Vec<Result<UnprocessedUserLine, InvalidJsonlError>>, DataReadError> {
    Ok(json_lines::<UnprocessedUserLine, _>(path)
        .map_err(|_| DataReadError::InvalidDataPathError {
            path: path.to_owned(),
        })?
        .map(|x| {
            x.map_err(|_| InvalidJsonlError {
                path: "test".to_string(),
            })
        })
        .collect::<Vec<_>>())
}

pub fn import_data_from_dir_with_res(
    data_dir: &str,
) -> Result<Vec<UnprocessedUserLine>, DataReadError> {
    let paths = read_dir(data_dir).map_err(|_| DataReadError::InvalidDataPathError {
        path: data_dir.to_string(),
    })?;

    paths
        .flatten()
        .filter(|paths| paths.path().extension().unwrap_or_default() == "jsonl")
        .map(|path| import_data_from_file_with_res(path.path().to_str().unwrap()))
        .fold_ok(Vec::<UnprocessedUserLine>::new(), |mut acc, mut x| {
            acc.append(&mut x);
            acc
        })
}

pub fn import_data_from_file_with_res(
    path: &str,
) -> Result<Vec<UnprocessedUserLine>, DataReadError> {
    let mut result: Vec<UnprocessedUserLine> = Vec::new();
    let lines_iter = json_lines::<UnprocessedUserLine, _>(path).map_err(|_| {
        DataReadError::InvalidDataPathError {
            path: path.to_string(),
        }
    })?;

    for line in lines_iter {
        let line = if let Ok(line) = line {
            line
        } else {
            return Err(DataReadError::InvalidJsonlError(InvalidJsonlError {
                path: path.to_string(),
            }));
        };

        result.push(line);
    }
    Ok(result)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn test_error_on_nonexisting_file() {
        let result = import_data_from_file_with_res("no-data-here");
        assert_eq!(
            result,
            Err(DataReadError::InvalidDataPathError {
                path: "no-data-here".to_string()
            })
        )
    }

    #[test]
    pub fn test_error_on_invalid_json_with_error_collect() {
        let result = import_data_from_file_with_res("data/invalid-data/data.jsonl");
        dbg!(&result);
        match result {
            Err(DataReadError::InvalidJsonlError(..)) => (),
            Err(_) => panic!(),
            Ok(_) => panic!(),
        }
    }
}
