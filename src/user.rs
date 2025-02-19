use crate::spam_score::SpamScore;
use chrono::DateTime;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_jsonlines::json_lines;
use std::fs::read_dir;
use thiserror::Error;

#[derive(Debug)]
pub struct User {
    fid: usize,
    labels: Vec<SpamRecord>,
}

type SpamRecord = (SpamScore, NaiveDate);

impl User {
    /// This method only takes a single SpamRecord as input. Therefore it cannot fail. Add more
    /// SpamRecords with add_spam_record. This function is mostly used for testing.
    pub fn new(fid: usize, labels: SpamRecord) -> Self {
        Self {
            fid,
            labels: vec![labels],
        }
    }

    pub fn fid(&self) -> usize {
        self.fid
    }

    pub fn latest_spam_record(&self) -> &SpamRecord {
        self.labels.last().unwrap()
    }

    pub fn earliest_spam_record(&self) -> &SpamRecord {
        self.labels.first().unwrap()
    }

    pub fn all_spam_records(&self) -> &Vec<SpamRecord> {
        &self.labels
    }

    pub fn created_at_or_after_date(&self, date: NaiveDate) -> bool {
        self.earliest_spam_record().1 >= date
    }

    pub fn created_at_or_before_date(&self, date: NaiveDate) -> bool {
        self.earliest_spam_record().1 <= date
    }

    fn add_spam_record(&mut self, new_record: SpamRecord) -> Result<(), UserError> {
        let mut label_iter = self.labels.iter().enumerate();
        let label_iter_len = self.labels.iter().len();

        let index = loop {
            if let Some((i, (score, date))) = label_iter.next() {
                if new_record.1 < *date {
                    break i;
                } else if new_record.1 == *date && new_record.0 == *score {
                    return Ok(());
                } else if new_record.1 == *date && new_record.0 != *score {
                    return Err(UserError::SpamScoreCollision {
                        fid: self.fid(),
                        date: *date,
                        old_spam_score: *score,
                        new_spam_score: new_record.0,
                    });
                };
            } else {
                break label_iter_len;
            };
        };

        self.labels.insert(index, new_record);
        Ok(())
    }

    /// Merges two user objects into one. Both must have the same FID and no contradictory spam
    /// records (i.e same date but different scores)
    pub fn merge_user(&mut self, other: Self) -> Result<(), UserError> {
        if self.fid != other.fid() {
            return Err(UserError::DifferentFidMerge {
                fid_1: self.fid,
                fid_2: other.fid,
            });
        };

        for spam_record in other.all_spam_records() {
            self.add_spam_record(*spam_record)?
        }

        Ok(())
    }

    #[deprecated(note = "use merge_user instead")]
    pub fn update_user(&mut self, other: Self) {
        assert_eq!(self.fid(), other.fid());
        //self.labels.push(*other.labels.first().unwrap())
        if let Err(err) = self.add_spam_record(*other.labels.first().unwrap()) {
            println!("{:?}", err);
        }
    }

    pub fn last_spam_score_update_date(&self) -> NaiveDate {
        self.labels.last().unwrap().1
    }

    pub fn earliest_spam_score_date(&self) -> NaiveDate {
        let mut labels_iter = self.labels.iter();

        let mut earliest_date = labels_iter.next().unwrap().1;
        for (_, date) in labels_iter {
            if date < &earliest_date {
                earliest_date = *date;
            }
        }
        earliest_date
    }

    /// If the user didn't exist at the date, the function returns none.
    pub fn spam_score_at_date(&self, date: &NaiveDate) -> Option<&SpamScore> {
        if date < &self.earliest_spam_record().1 {
            return None;
        } else if date >= &self.latest_spam_record().1 {
            return Some(&self.latest_spam_record().0);
        };

        let mut labels_iter = self.labels.iter();

        let result_spam_record = loop {
            if let Some(current_spam_record) = labels_iter.next() {
                if current_spam_record.1 <= *date {
                    break current_spam_record;
                }
            } else {
                panic!();
            }
        };

        Some(&result_spam_record.0)
    }
}

#[derive(Error, Debug)]
pub enum UserError {
    #[error("User {0} already has a spam record for date {1}. The existing spam score at the date is {} but a spamscore of {} is now trying to be set.", .fid, . date) ]
    SpamScoreCollision {
        fid: usize,
        date: NaiveDate,
        old_spam_score: SpamScore,
        new_spam_score: SpamScore,
    },
    #[error("Trying to merge users with different fids. For merge_user to work both input users must have the same fid. provided fid_1: {} and provided fid_2 {}", .fid_1, .fid_2)]
    DifferentFidMerge { fid_1: usize, fid_2: usize },
}

impl TryFrom<UnprocessedUserLine> for User {
    type Error = InvalidInputError;

    fn try_from(value: UnprocessedUserLine) -> Result<Self, Self::Error> {
        let label_value = SpamScore::try_from(value.label_value)?;
        let fid = value.fid();
        let date =
            if let Some(date) = DateTime::from_timestamp(value.timestamp.try_into().unwrap(), 0) {
                date.date_naive()
            } else {
                return Err(InvalidInputError::DateError {
                    timestamp: value.timestamp,
                });
            };

        let labels: Vec<(SpamScore, NaiveDate)> = vec![(label_value, date)];

        Ok(Self { fid, labels })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Type {
    fid: u64,
    target: String,
}

#[derive(Error, Debug)]
#[error("Input data is not jsonl at : .path")]
pub struct InvalidJsonlError {
    path: String,
}

#[derive(Error, Debug)]
pub enum DataReadError {
    #[error("Input data is not jsonl at : .path")]
    InvalidJsonlError(#[from] InvalidJsonlError),

    #[error("The path {0} is invalid", .path)]
    InvalidDataPathError { path: String },
}

#[derive(Error, Debug)]
pub enum InvalidInputError {
    #[error("SpamScore was {0}, not zero, one or two.", .label)]
    SpamScoreError { label: usize },
    #[error("Timestamp was {0}, which is invalid.", . timestamp)]
    DateError { timestamp: usize },
}

#[derive(Serialize, Deserialize, Debug)]
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

    #[deprecated(note = "use import_data_from_file_with_res")]
    pub fn import_data_from_file(path: &str) -> Vec<UnprocessedUserLine> {
        json_lines(path)
            .unwrap()
            .collect::<Result<Vec<UnprocessedUserLine>, _>>()
            .unwrap()
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
                    path: "hello".to_string(),
                }));
            };

            result.push(line);
        }
        Ok(result)
    }

    pub fn import_data_from_dir_with_res(
        data_dir: &str,
    ) -> Result<Vec<UnprocessedUserLine>, DataReadError> {
        let paths = if let Ok(paths) = read_dir(data_dir) {
            paths
        } else {
            return Err(DataReadError::InvalidDataPathError {
                path: data_dir.to_string(),
            });
        };

        let mut result: Vec<UnprocessedUserLine> = Vec::new();

        for path in paths.flatten() {
            if path.path().extension().unwrap_or_default() == "jsonl" {
                let new_jsonl_lines =
                    if let Ok(lines) = json_lines::<UnprocessedUserLine, _>(path.path()) {
                        lines
                    } else {
                        return Err(DataReadError::InvalidJsonlError(InvalidJsonlError {
                            path: path.path().to_str().unwrap().to_owned(),
                        }));
                    };

                for line in new_jsonl_lines {
                    result.push(if let Ok(line) = line {
                        line
                    } else {
                        return Err(DataReadError::InvalidJsonlError(InvalidJsonlError {
                            path: path.path().to_str().unwrap().to_owned(),
                        }));
                    })
                }
            }
        }
        Ok(result)
    }

    //TODO it's probably must for efficient to check the dates of the first line of each file and
    //abort when it reaches a data that has already been checked. There is probably a lot of
    //duplicate checking right now.
    #[deprecated(note = "use import_data_from_dir_with_res instead")]
    pub fn import_data_from_dir(data_dir: &str) -> Vec<UnprocessedUserLine> {
        let paths = read_dir(data_dir).unwrap();
        let mut result: Vec<UnprocessedUserLine> = Vec::new();

        for path in paths.flatten() {
            if path.path().extension().unwrap_or_default() == "jsonl" {
                let mut new_lines = json_lines(path.path())
                    .unwrap()
                    .collect::<Result<Vec<UnprocessedUserLine>, _>>()
                    .unwrap();
                result.append(&mut new_lines);
            }
        }
        result
    }
}

#[cfg(test)]
#[allow(deprecated)]
pub mod tests {
    use super::*;
    use crate::user_collection::UserCollection;

    #[test]
    pub fn test_label_value_invalid_input() {
        assert!(SpamScore::try_from(0).is_ok());
        assert!(SpamScore::try_from(1).is_ok());
        assert!(SpamScore::try_from(2).is_ok());
        assert!(SpamScore::try_from(3).is_err());
        assert!(SpamScore::try_from(100).is_err());
    }

    #[test]
    pub fn test_spam_score_collision_error_for_invalid_record_add() {
        let date = NaiveDate::from_ymd_opt(2020, 1, 2).unwrap();
        let later_date = NaiveDate::from_ymd_opt(2020, 1, 3).unwrap();
        let earlier_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

        let spam_record = (SpamScore::One, date);
        let mut user = User {
            fid: 1,
            labels: vec![spam_record],
        };

        assert!(user.add_spam_record((SpamScore::Zero, date)).is_err());
        assert!(user.add_spam_record((SpamScore::One, date)).is_ok());
        assert_eq!(user.all_spam_records().len(), 1);
        assert!(user.add_spam_record((SpamScore::Two, later_date)).is_ok());
        assert_eq!(user.all_spam_records().len(), 2);

        //make sure spam_records are sorted
        assert_eq!(user.all_spam_records().first().unwrap().1, date);
        assert_eq!(user.all_spam_records().last().unwrap().1, later_date);

        assert!(user
            .add_spam_record((SpamScore::Zero, earlier_date))
            .is_ok());
        assert_eq!(
            user.all_spam_records().first().unwrap(),
            &(SpamScore::Zero, earlier_date)
        );
        assert_eq!(user.all_spam_records()[1], (SpamScore::One, date));
        assert_eq!(
            user.all_spam_records().last().unwrap(),
            &(SpamScore::Two, later_date)
        );
    }

    #[test]
    pub fn test_dummy_data_import_with_new() {
        let fid = 1;
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        assert_eq!(users.spam_score_by_fid(fid).unwrap(), SpamScore::Zero);
        let user = users.user(fid).unwrap();
        assert_eq!(
            user.earliest_spam_score_date(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );

        assert_eq!(
            user.last_spam_score_update_date(),
            NaiveDate::from_ymd_opt(2025, 1, 23).unwrap()
        );
    }

    #[test]
    #[allow(deprecated)]
    pub fn test_dummy_data_import() {
        let fid = 1;
        let users = UserCollection::create_from_dir("data/dummy-data");
        assert_eq!(users.spam_score_by_fid(fid).unwrap(), SpamScore::Zero);
        let user = users.user(fid).unwrap();
        assert_eq!(
            user.earliest_spam_score_date(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );

        assert_eq!(
            user.last_spam_score_update_date(),
            NaiveDate::from_ymd_opt(2025, 1, 23).unwrap()
        );
    }

    #[test]
    pub fn test_user_created_after_date_on_dummy_data_with_new() {
        let fid = 1;
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();

        assert!(users.user(fid).unwrap().created_at_or_after_date(date));

        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(users.user(fid).unwrap().created_at_or_after_date(date));

        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        assert!(!users.user(fid).unwrap().created_at_or_after_date(date));

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert!(!users.user(fid).unwrap().created_at_or_after_date(date));
    }

    #[test]
    #[allow(deprecated)]
    pub fn test_user_created_after_date_on_dummy_data() {
        let fid = 1;
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let users = UserCollection::create_from_dir("data/dummy-data");

        assert!(users.user(fid).unwrap().created_at_or_after_date(date));

        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(users.user(fid).unwrap().created_at_or_after_date(date));

        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        assert!(!users.user(fid).unwrap().created_at_or_after_date(date));

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert!(!users.user(fid).unwrap().created_at_or_after_date(date));
    }

    #[test]
    pub fn test_spam_score_by_date_on_dummy_data_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let date = NaiveDate::from_ymd_opt(2023, 1, 25).unwrap();
        let user = users.user(1).unwrap();

        assert!(user.spam_score_at_date(&date).is_none());

        let user = users.user(1).unwrap();

        let date = NaiveDate::from_ymd_opt(2024, 1, 25).unwrap();
        assert_eq!(user.spam_score_at_date(&date).unwrap(), &SpamScore::One);

        let date = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();
        assert_eq!(user.spam_score_at_date(&date).unwrap(), &SpamScore::One);

        let date = NaiveDate::from_ymd_opt(2025, 1, 23).unwrap();
        assert_eq!(user.spam_score_at_date(&date).unwrap(), &SpamScore::Zero);

        let date = NaiveDate::from_ymd_opt(2025, 1, 25).unwrap();
        assert_eq!(user.spam_score_at_date(&date).unwrap(), &SpamScore::Zero);
    }

    #[test]
    #[allow(deprecated)]
    pub fn test_spam_score_by_date_on_dummy_data() {
        let users = UserCollection::create_from_dir("data/dummy-data");
        let date = NaiveDate::from_ymd_opt(2023, 1, 25).unwrap();
        let user = users.user(1).unwrap();

        assert!(user.spam_score_at_date(&date).is_none());

        let user = users.user(1).unwrap();

        let date = NaiveDate::from_ymd_opt(2024, 1, 25).unwrap();
        assert_eq!(user.spam_score_at_date(&date).unwrap(), &SpamScore::One);

        let date = NaiveDate::from_ymd_opt(2025, 1, 20).unwrap();
        assert_eq!(user.spam_score_at_date(&date).unwrap(), &SpamScore::One);

        let date = NaiveDate::from_ymd_opt(2025, 1, 23).unwrap();
        assert_eq!(user.spam_score_at_date(&date).unwrap(), &SpamScore::Zero);

        let date = NaiveDate::from_ymd_opt(2025, 1, 25).unwrap();
        assert_eq!(user.spam_score_at_date(&date).unwrap(), &SpamScore::Zero);
    }

    #[test]
    fn test_created_by_before_date_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();

        let user = users.user(1).unwrap();
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()));

        let user = users.user(2).unwrap();
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()));
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()));
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()));
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 1, 22).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 1, 23).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 1, 24).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()));
    }

    #[test]
    #[allow(deprecated)]
    fn test_created_by_before_date() {
        let users = UserCollection::create_from_dir("data/dummy-data");

        let user = users.user(1).unwrap();
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()));

        let user = users.user(2).unwrap();
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2023, 1, 31).unwrap()));
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()));
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap()));
        assert!(!user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 1, 22).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 1, 23).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 1, 24).unwrap()));
        assert!(user.created_at_or_before_date(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()));
    }
}
