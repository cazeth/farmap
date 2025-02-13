use crate::spam_score::SpamScore;
use crate::utils::distribution_from_counts;
use chrono::DateTime;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_jsonlines::json_lines;
use std::collections::HashMap;
use std::fs::read_dir;
use thiserror::Error;

#[derive(Debug)]
pub struct User {
    fid: usize,
    labels: Vec<SpamRecord>,
}

type SpamRecord = (SpamScore, NaiveDate);

impl User {
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

    // TODO need to return an error if a spam record exists with the same date.
    fn add_spam_record(&mut self, new_record: SpamRecord) -> Result<(), String> {
        //let mut index = 0;
        let mut label_iter = self.labels.iter().enumerate();
        let label_iter_len = self.labels.iter().len();

        let index = loop {
            if let Some((i, (_, label_date))) = label_iter.next() {
                if new_record.1 < *label_date {
                    break i;
                };
            } else {
                break label_iter_len;
            };
        };

        self.labels.insert(index, new_record);
        Ok(())
    }

    pub fn merge_user(&mut self, other: Self) -> Result<(), String> {
        assert_eq!(self.fid(), other.fid());

        for spam_record in other.labels {
            self.add_spam_record(spam_record)?;
        }

        todo!();
    }

    // TODO : this method is weird because if other has many elements in label it doesn't merge
    // all of them but it still accepts it as input.
    // it's more of a merge but it doesn't quite work.
    pub fn update_user(&mut self, other: Self) {
        assert_eq!(self.fid(), other.fid());
        //self.labels.push(*other.labels.first().unwrap())
        self.add_spam_record(*other.labels.first().unwrap())
            .unwrap();
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

#[derive(Default)]
pub struct Users {
    map: HashMap<usize, User>,
}

impl Users {
    /// add a user to the collection. If the fid already exists, the label is updated.
    pub fn push(&mut self, user: User) -> bool {
        if let Some(existing_user) = self.map.get_mut(&user.fid()) {
            existing_user.update_user(user);
            false
        } else {
            self.map.insert(user.fid(), user);
            true
        }
    }

    /// Return Some<SpamScore> if the fid exists, otherwise returns none.
    pub fn spam_score_by_fid(&self, fid: usize) -> Option<SpamScore> {
        let user = self.map.get(&fid)?;
        Some(user.latest_spam_record().0)
    }

    pub fn user(&self, fid: usize) -> Option<&User> {
        self.map.get(&fid)
    }

    pub fn user_count(&self) -> usize {
        self.map.len()
    }

    pub fn user_count_at_date(&self, date: NaiveDate) -> usize {
        self.map
            .iter()
            .filter(|(_, user)| user.spam_score_at_date(&date).is_some())
            .count()
    }

    pub fn create_from_dir(dir: &str) -> Self {
        let unprocessed_user_line = UnprocessedUserLine::import_data_from_dir(dir);
        let mut users = Users::default();
        for line in unprocessed_user_line {
            users.push(User::try_from(line).unwrap());
        }
        users
    }

    pub fn create_from_file(path: &str) -> Self {
        let mut users = Users::default();
        let unprocessed_user_line = UnprocessedUserLine::import_data_from_file(path);

        for line in unprocessed_user_line {
            users.push(User::try_from(line).unwrap());
        }

        users
    }

    /// Applies a filter to the user data. Use with caution since the data is removed from the
    /// struct. For most situations it is preferred to create a subset of the data.
    pub fn apply_filter<F>(&mut self, filter: F)
    where
        F: Fn(&User) -> bool,
    {
        let old_map = std::mem::take(&mut self.map);
        let new_map = old_map
            .into_values()
            .filter(|user| filter(user))
            .map(|user| (user.fid(), user))
            .collect::<HashMap<usize, User>>();
        self.map = new_map;
    }

    /// Returns the distribution of spam scores at a certain date. Excludes users that did not
    /// exist at the given date.
    /// Returns none if the struct contains no users
    pub fn spam_score_distribution_at_date(&self, date: NaiveDate) -> Option<[f32; 3]> {
        let mut counts = [0; 3];

        for spam_score in self
            .map
            .iter()
            .filter_map(|(_, user)| user.spam_score_at_date(&date))
        {
            match spam_score {
                SpamScore::Zero => counts[0] += 1,
                SpamScore::One => counts[1] += 1,
                SpamScore::Two => counts[2] += 1,
            }
        }

        distribution_from_counts(&counts)
    }

    /// Returns the spam_score_distribution after applying a filter. The function returns None if
    /// the subset is empty.
    pub fn spam_score_distribution_for_subset<F>(&self, filter: F) -> Option<[f32; 3]>
    where
        F: Fn(&User) -> bool,
    {
        let mut counts = [0; 3];

        for user in self.map.values().filter(|user| filter(user)) {
            match user.latest_spam_record().0 {
                SpamScore::Zero => counts[0] += 1,
                SpamScore::One => counts[1] += 1,
                SpamScore::Two => counts[2] += 2,
            }
        }

        distribution_from_counts(&counts)
    }

    pub fn current_spam_score_distribution(&self) -> Option<[f32; 3]> {
        let mut counts = [0; 3];
        for (_, user) in self.map.iter() {
            match user.latest_spam_record().0 {
                SpamScore::Zero => counts[0] += 1,
                SpamScore::One => counts[1] += 1,
                SpamScore::Two => counts[2] += 1,
            }
        }

        distribution_from_counts(&counts)
    }

    pub fn iter(&self) -> impl Iterator<Item = &User> {
        self.map.values()
    }
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

    pub fn import_data_from_file(path: &str) -> Vec<UnprocessedUserLine> {
        json_lines(path)
            .unwrap()
            .collect::<Result<Vec<UnprocessedUserLine>, _>>()
            .unwrap()
    }

    //TODO it's probably must for efficient to check the dates of the first line of each file and
    //abort when it reaches a data that has already been checked. There is probably a lot of
    //duplicate checking right now.
    pub fn import_data_from_dir(data_dir: &str) -> Vec<UnprocessedUserLine> {
        let paths = read_dir(data_dir).unwrap();
        let mut result: Vec<UnprocessedUserLine> = Vec::new();
        println!("paths :{:?}", paths);

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
pub mod tests {

    use super::*;

    #[test]
    pub fn test_label_value_invalid_input() {
        assert!(SpamScore::try_from(0).is_ok());
        assert!(SpamScore::try_from(1).is_ok());
        assert!(SpamScore::try_from(2).is_ok());
        assert!(SpamScore::try_from(3).is_err());
        assert!(SpamScore::try_from(100).is_err());
    }

    #[test]
    pub fn test_user_count_on_file() {
        let users = Users::create_from_file("data/dummy-data/spam.jsonl");
        assert_eq!(users.user_count(), 2);
        let users = Users::create_from_dir("data/dummy-data/");
        assert_eq!(users.user_count(), 2);
    }

    #[test]
    pub fn test_user_count_on_dir() {
        let users = Users::create_from_dir("data/dummy-data/");
        assert_eq!(users.user_count(), 2);
    }

    #[test]
    pub fn test_user_count_at_date() {
        let users = Users::create_from_dir("data/dummy-data/");
        assert_eq!(
            users.user_count_at_date(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()),
            0
        );

        assert_eq!(
            users.user_count_at_date(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()),
            0
        );

        assert_eq!(
            users.user_count_at_date(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            1
        );
        assert_eq!(
            users.user_count_at_date(NaiveDate::from_ymd_opt(2024, 5, 1).unwrap()),
            1
        );
        assert_eq!(
            users.user_count_at_date(NaiveDate::from_ymd_opt(2025, 5, 1).unwrap()),
            2
        );
    }

    #[test]
    pub fn test_dummy_data_import() {
        let fid = 1;
        let users = Users::create_from_dir("data/dummy-data");
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
    pub fn test_user_created_after_date_on_dummy_data() {
        let fid = 1;
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let users = Users::create_from_dir("data/dummy-data");

        assert!(users.user(fid).unwrap().created_at_or_after_date(date));

        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        assert!(users.user(fid).unwrap().created_at_or_after_date(date));

        let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        assert!(!users.user(fid).unwrap().created_at_or_after_date(date));

        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert!(!users.user(fid).unwrap().created_at_or_after_date(date));
    }

    #[test]
    pub fn test_spam_score_by_date_on_dummy_data() {
        let users = Users::create_from_dir("data/dummy-data");
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
    fn test_spam_distribution_for_users_created_at_or_after_date() {
        let users = Users::create_from_dir("data/dummy-data");
        let date = NaiveDate::from_ymd_opt(2025, 1, 23).unwrap();
        let closure = |user: &User| user.created_at_or_after_date(date);

        assert_eq!(
            users.spam_score_distribution_for_subset(closure),
            Some([0.0, 0.0, 1.0])
        );
    }

    #[test]
    fn test_spam_distribution_for_one_fid() {
        let users = Users::create_from_dir("data/dummy-data");
        let closure = |user: &User| user.fid() == 2;

        assert_eq!(
            users.spam_score_distribution_for_subset(closure),
            Some([0.0, 0.0, 1.0])
        );
    }

    #[test]
    fn test_apply_filter_for_one_fid() {
        let mut users = Users::create_from_dir("data/dummy-data");
        let closure = |user: &User| user.fid() == 2;
        users.apply_filter(closure);
        assert_eq!(
            users.current_spam_score_distribution(),
            Some([0.0, 0.0, 1.0])
        )
    }

    #[test]
    fn test_none_for_filtered_spam_distribution() {
        let users = Users::create_from_dir("data/dummy-data");
        let closure = |user: &User| user.fid() == 3;

        assert_eq!(users.spam_score_distribution_for_subset(closure), None);
    }

    #[test]
    fn test_created_by_before_date() {
        let users = Users::create_from_dir("data/dummy-data");

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
