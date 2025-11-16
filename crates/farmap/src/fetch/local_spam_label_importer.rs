//! Import data from local files with spam label data.
//! The data can be added to a [UserCollection](crate::UserCollection).
use super::{DataReadError, RetrieveError};
use crate::fetch::InvalidJsonlError;
use crate::spam_score::DatedSpamUpdate;
use crate::Fidded;
use crate::UnprocessedUserLine;
use itertools::Itertools;
use serde_jsonlines::json_lines;
use std::fs::read_dir;
use std::path::Path;

pub fn import_data_from_file(
    path: impl AsRef<Path>,
) -> Result<Vec<Fidded<DatedSpamUpdate>>, RetrieveError> {
    let import_result = import_data_from_file_with_collected_res(path)
        .map_err(|_| RetrieveError::CouldNotFetchData)?;

    import_result
        .into_iter()
        .flatten()
        .map(TryInto::<Fidded<DatedSpamUpdate>>::try_into)
        .try_collect()
        .map_err(|_| RetrieveError::InvalidFetchedData)
}

pub fn import_data_from_file_with_collected_res(
    path: impl AsRef<Path>,
) -> Result<Vec<Result<UnprocessedUserLine, InvalidJsonlError>>, DataReadError> {
    let path_ref = path.as_ref();
    Ok(json_lines::<UnprocessedUserLine, _>(&path)
        .map_err(|_| DataReadError::InvalidDataPathError {
            path: path_ref.to_str().unwrap().to_string(),
        })?
        .map(|x| {
            x.map_err(|_| InvalidJsonlError {
                path: path_ref.to_str().unwrap_or_default().to_string(),
            })
        })
        .collect::<Vec<_>>())
}

pub fn import_data_from_dir_with_collected_res(
    path: impl AsRef<Path>,
) -> Result<Vec<Result<UnprocessedUserLine, InvalidJsonlError>>, DataReadError> {
    let path_ref = path.as_ref();

    let paths = read_dir(&path).map_err(|_| DataReadError::InvalidDataPathError {
        path: path_ref.to_str().unwrap_or_default().to_string(),
    })?;
    dbg!(&paths);
    let file_results = paths
        .flatten()
        .filter(|paths| paths.path().extension().unwrap_or_default() == "jsonl")
        .map(|path| import_data_from_file_with_collected_res(path.path()))
        .collect_vec();

    let mut result: Vec<Result<UnprocessedUserLine, InvalidJsonlError>> = Vec::new();

    for file_result in file_results {
        if let Ok(mut file_result) = file_result {
            result.append(&mut file_result);
        } else {
            return Err(DataReadError::InvalidDataPathError {
                path: path_ref.to_str().unwrap_or_default().to_string(),
            });
        }
    }
    Ok(result)
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
    let import_results = import_data_from_file_with_collected_res(path)?;
    let result: Result<_, DataReadError> = import_results
        .into_iter()
        .try_collect()
        .map_err(DataReadError::InvalidJsonlError);
    result
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

    #[test]
    pub fn test_error_collect_on_invalid_jsonl() {
        let result = import_data_from_dir_with_collected_res("data/invalid-data/");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().iter().filter(|x| x.is_err()).count(), 1);
    }

    #[test]
    pub fn test_valid_jsonl() {
        let result = import_data_from_dir_with_collected_res("data/dummy-data/");
        println!("{result:?}");
        assert!(result.is_ok());
        assert!(result.unwrap().iter().all(|x| x.is_ok()));
    }
}
