use crate::fetch::DataReadError;
use crate::fetch::InvalidJsonlError;
use crate::spam_score::DatedSpamUpdate;
use crate::spam_score::SpamScoreError;
use crate::Fidded;
use crate::SpamScore;
use chrono::DateTime;
use chrono::NaiveDate;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_jsonlines::json_lines;
use std::fs::read_dir;
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct UnprocessedUserLine {
    provider: usize,
    r#type: Type,
    label_type: String,
    label_value: usize,
    timestamp: usize,
}

impl UnprocessedUserLine {
    pub fn provider(&self) -> usize {
        self.provider
    }

    pub fn fid(&self) -> usize {
        self.r#type.fid as usize
    }

    pub fn label_value(&self) -> usize {
        self.label_value
    }

    pub fn timestamp(&self) -> usize {
        self.timestamp
    }

    pub fn date(&self) -> Result<NaiveDate, SpamDataParseError> {
        if let Some(date) = DateTime::from_timestamp(self.timestamp().try_into().unwrap(), 0) {
            Ok(date.date_naive())
        } else {
            Err(SpamDataParseError::DateError {
                timestamp: self.timestamp(),
            })
        }
    }

    #[deprecated(note = "use local_spam_label_importer instead")]
    #[doc(hidden)]
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

    /// collects error on a line-by-line basis and sends them with an ok. Other fatal errors invoke
    /// an error.
    #[deprecated(note = "use local_spam_label_importer instead")]
    #[doc(hidden)]
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

    #[deprecated(note = "use local_spam_label_importer instead")]
    #[allow(deprecated)]
    #[doc(hidden)]
    pub fn import_data_from_dir_with_res(
        data_dir: &str,
    ) -> Result<Vec<UnprocessedUserLine>, DataReadError> {
        let paths = read_dir(data_dir).map_err(|_| DataReadError::InvalidDataPathError {
            path: data_dir.to_string(),
        })?;

        paths
            .flatten()
            .filter(|paths| paths.path().extension().unwrap_or_default() == "jsonl")
            .map(|path| Self::import_data_from_file_with_res(path.path().to_str().unwrap()))
            .fold_ok(Vec::<UnprocessedUserLine>::new(), |mut acc, mut x| {
                acc.append(&mut x);
                acc
            })
    }
}

impl TryFrom<UnprocessedUserLine> for Fidded<DatedSpamUpdate> {
    type Error = SpamDataParseError;
    fn try_from(value: UnprocessedUserLine) -> Result<Self, Self::Error> {
        let fid = value.fid();
        let date = value.date()?;
        value.label_value();
        let spam_score = SpamScore::try_from(value.label_value())?;

        let dated_spam_update = DatedSpamUpdate::from(date, spam_score);
        let fidded: Fidded<DatedSpamUpdate> = Fidded::from((dated_spam_update, fid));
        Ok(fidded)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct Type {
    fid: u64,
    target: String,
}

#[derive(Error, Debug, PartialEq)]
pub enum SpamDataParseError {
    #[error(transparent)]
    SpamScoreError(#[from] SpamScoreError),
    #[error("Timestamp was {0}, which is invalid.", . timestamp)]
    DateError { timestamp: usize },
}
