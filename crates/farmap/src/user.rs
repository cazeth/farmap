use crate::cast_meta::CastMeta;
use crate::spam_score::SpamScore;
use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use itertools::*;
use serde::{Deserialize, Serialize};
use serde_jsonlines::json_lines;
use std::fs::read_dir;
use thiserror::Error;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    fid: usize,
    labels: Vec<SpamRecord>,

    /// Some(Empty vec): has been checked and there were no cast records.
    /// None: Has not been checked.
    cast_records: Option<Vec<CastMeta>>,
    reaction_times: Option<Vec<NaiveDateTime>>,
    latest_reaction_time_update_date: Option<NaiveDateTime>,
    latest_cast_record_check_date: Option<NaiveDate>,
}

type SpamRecord = (SpamScore, NaiveDate);

impl User {
    /// This method only takes a single SpamRecord as input. Therefore it cannot fail. Add more
    /// SpamRecords with add_spam_record. This function is mostly used for testing.
    pub fn new(fid: usize, labels: SpamRecord) -> Self {
        Self {
            fid,
            labels: vec![labels],
            cast_records: None,
            latest_cast_record_check_date: None,
            reaction_times: None,
            latest_reaction_time_update_date: None,
        }
    }

    pub fn add_cast_records(&mut self, records: Vec<CastMeta>, check_date: NaiveDate) {
        self.cast_records = Some(records);
        self.latest_cast_record_check_date = Some(check_date);
    }

    #[deprecated(
        note = "use update rection times instead, as it prevents acccidentally overriding old values"
    )]
    pub fn add_reaction_times(&mut self, reaction_times: Vec<NaiveDateTime>) {
        self.reaction_times = Some(reaction_times);
    }

    pub fn update_reaction_times(
        &mut self,
        reaction_times: Vec<NaiveDateTime>,
    ) -> Option<Vec<NaiveDateTime>> {
        self.latest_reaction_time_update_date = Some(Local::now().naive_utc());
        self.reaction_times.replace(reaction_times)
    }

    /// Returns the fid of the user
    /// # Examples
    /// ```rust
    /// use farmap::User;
    /// use chrono::NaiveDate;
    /// use farmap::SpamScore;
    ///
    /// let user = User::new(1, (SpamScore::Zero, NaiveDate::from_ymd_opt(2020,1,1).unwrap()) );
    /// assert_eq!(user.fid(), 1);
    /// ````
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

    pub fn latest_reaction_time_update_date(&self) -> Option<NaiveDateTime> {
        self.latest_reaction_time_update_date
    }

    /// Adds a new spam record to a user. There are three scenarios that may happen:
    ///
    /// The user already has a spam record with the same date and the same record. The method
    /// returns Ok without doing anything.
    ///
    /// The user already has a spam record with a different spam score at the same date.
    /// The method does not change the struct and returns an error.
    ///
    /// There is no collision and the list is updated while remaining sorted.
    ///
    fn add_spam_record(&mut self, new_record: SpamRecord) -> Result<(), UserError> {
        let position_closest_and_smaller: Option<usize> =
            self.labels.iter().position(|(_, d)| *d >= new_record.1);

        let record_closest_and_smaller = position_closest_and_smaller.map(|p| self.labels[p]);

        match record_closest_and_smaller {
            Some((value, date)) if date == new_record.1 && value == new_record.0 => Ok(()),
            None => {
                self.labels.push(new_record);
                Ok(())
            }
            Some((_, date)) if date != new_record.1 => {
                self.labels
                    .insert(position_closest_and_smaller.unwrap(), new_record);
                Ok(())
            }
            Some(_) => Err(UserError::SpamScoreCollision {
                fid: self.fid,
                date: new_record.1,
                old_spam_score: record_closest_and_smaller.unwrap().0,
                new_spam_score: new_record.0,
            }),
        }
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

    pub fn cast_count(&self) -> Option<u64> {
        Some(self.cast_records.as_ref()?.len() as u64)
    }

    pub fn reaction_times(&self) -> &Option<Vec<NaiveDateTime>> {
        &self.reaction_times
    }

    pub fn average_monthly_cast_rate(&self) -> Option<f32> {
        let [sum, count] = self
            .monthly_cast_counts()?
            .iter()
            .fold([0, 0], |acc, (x, _)| [acc[0] + x, acc[1] + 1]);

        Some(sum as f32 / count as f32)
    }

    pub fn monthly_cast_counts(&self) -> Option<Vec<(usize, NaiveDate)>> {
        let cast_records = if let Some(cast_records) = &self.cast_records {
            cast_records
        } else {
            return None;
        };

        Some(
            cast_records
                .iter()
                .map(|x| {
                    NaiveDate::from_ymd_opt(x.cast_date().year(), x.cast_date().month(), 1)
                        .expect("date parsing inside monthly_cast_count should work")
                })
                .sorted()
                .dedup_with_count()
                .collect::<Vec<_>>(),
        )
    }

    pub fn has_cast_data(&self) -> bool {
        self.cast_records.is_some()
    }

    pub fn latest_cast_record_check_date(&self) -> Option<NaiveDate> {
        self.latest_cast_record_check_date
    }

    #[doc(hidden)]
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
        *self.labels.iter().map(|(_, date)| date).min().unwrap()
    }

    /// If the user didn't exist at the date, the function returns none.
    pub fn spam_score_at_date(&self, date: &NaiveDate) -> Option<&SpamScore> {
        if date < &self.earliest_spam_record().1 {
            return None;
        };
        self.labels
            .iter()
            .rev()
            .find(|(_, d)| d <= date)
            .map(|(score, _)| score)
    }
}

#[derive(Error, Debug, PartialEq)]
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

        Ok(Self {
            fid,
            labels,
            cast_records: None,
            latest_cast_record_check_date: None,
            reaction_times: None,
            latest_reaction_time_update_date: None,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Type {
    fid: u64,
    target: String,
}

#[derive(Error, Debug, PartialEq)]
#[error("Input data is not jsonl at : .path")]
pub struct InvalidJsonlError {
    path: String,
}

#[derive(Error, Debug, PartialEq)]
pub enum DataReadError {
    #[error("Input data is not jsonl at : .path")]
    InvalidJsonlError(#[from] InvalidJsonlError),

    #[error("The path {0} is invalid", .path)]
    InvalidDataPathError { path: String },
}

#[derive(Error, Debug, PartialEq)]
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
                    path: path.to_string(),
                }));
            };

            result.push(line);
        }
        Ok(result)
    }

    /// collects error on a line-by-line basis and sends them with an ok. Other fatal errors invoke
    /// an error.
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
            .map(|path| Self::import_data_from_file_with_res(path.path().to_str().unwrap()))
            .fold_ok(Vec::<UnprocessedUserLine>::new(), |mut acc, mut x| {
                acc.append(&mut x);
                acc
            })
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
            cast_records: None,
            latest_cast_record_check_date: None,
            reaction_times: None,
            latest_reaction_time_update_date: None,
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
