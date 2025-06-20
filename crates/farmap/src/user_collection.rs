use crate::spam_score::SpamScore;
use crate::user::DataReadError;
use crate::user::InvalidInputError;
use crate::user::UnprocessedUserLine;
use crate::user::User;
use crate::user::UserError;
use crate::utils::distribution_from_counts;
use chrono::NaiveDate;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use thiserror::Error;

#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserCollection {
    map: HashMap<usize, User>,
}

type CreateResult = Result<(UserCollection, Vec<DataCreationError>), DataCreationError>;

impl UserCollection {
    /// add a user to the collection. If the fid already exists, the label is updated.
    /// This method may fail if the user is considered invalid in UserCollection because of
    /// SpamScoreCollision.
    pub fn push_with_res(&mut self, user: User) -> Result<bool, UserError> {
        if let Some(existing_user) = self.map.get_mut(&user.fid()) {
            existing_user.merge_user(user)?;
            Ok(false)
        } else {
            self.map.insert(user.fid(), user);
            Ok(true)
        }
    }

    /// Return `Some(SpamScore)` if the fid exists, otherwise returns none.
    /// Return None if the user if exists but has no spam record.
    pub fn spam_score_by_fid(&self, fid: usize) -> Option<SpamScore> {
        let user = self.map.get(&fid)?;
        Some(user.latest_spam_record_with_opt()?.0)
    }

    pub fn user_mut(&mut self, fid: usize) -> Option<&mut User> {
        self.map.get_mut(&fid)
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

    pub fn create_from_dir_with_res(dir: &str) -> Result<Self, DataCreationError> {
        let unprocessed_user_line = UnprocessedUserLine::import_data_from_dir_with_res(dir)?;
        let mut users = UserCollection::default();
        for line in unprocessed_user_line {
            users.push_with_res(User::try_from(line)?)?;
        }
        Ok(users)
    }

    /// A data importer that keeps running in case of nonfatal errors.
    /// Nonfatal errors are spam collision errors or invalid parameter data. In case of such error
    /// the import continues to run and returns the errors in a vec alongside the return data.
    pub fn create_from_dir_and_collect_non_fatal_errors(dir: &str) -> CreateResult {
        // these errors are considered fatal for now.
        let lines = UnprocessedUserLine::import_data_from_dir_with_res(dir)?;

        Ok(UserCollection::create_from_unprocessed_user_lines_and_collect_non_fatal_errors(lines))
    }

    /// Like create_from_dir ... but for a single file.
    pub fn create_from_file_and_collect_non_fatal_errors(file: &str) -> CreateResult {
        let lines = UnprocessedUserLine::import_data_from_file_with_res(file)?;

        Ok(UserCollection::create_from_unprocessed_user_lines_and_collect_non_fatal_errors(lines))
    }

    // the problem with this is that when the file does not exist the program will fail because
    // there isn't really a way for the caller to anticipate this...
    pub fn create_from_db(db: &Path) -> Result<Self, DbReadError> {
        Ok(serde_json::from_str(&std::fs::read_to_string(db)?)?)
    }

    pub fn create_from_file(file: &mut std::fs::File) -> Result<Self, DbReadError> {
        let mut result = String::new();
        file.read_to_string(&mut result)?;
        Ok(serde_json::from_str(&result)?)
    }

    pub fn save_to_db(&self, db: &Path) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(db)?;
        let json_text = serde_json::to_string(self)?;
        file.write_all(json_text.as_bytes())?;
        Ok(())
    }

    pub fn push_unprocessed_user_line(
        &mut self,
        line: UnprocessedUserLine,
    ) -> Result<(), Box<dyn Error>> {
        let new_user = User::try_from(line)?;
        self.push_with_res(new_user)?;
        Ok(())
    }

    fn create_from_unprocessed_user_lines_and_collect_non_fatal_errors(
        lines: Vec<UnprocessedUserLine>,
    ) -> (UserCollection, Vec<DataCreationError>) {
        let mut users = UserCollection::default();

        let mut non_fatal_errors: Vec<DataCreationError> = Vec::new();

        for line in lines {
            let user = match User::try_from(line) {
                Ok(user) => user,
                Err(err) => {
                    non_fatal_errors.push(DataCreationError::InvalidInputError(err));
                    continue;
                }
            };

            if let Err(err) = users.push_with_res(user) {
                non_fatal_errors.push(DataCreationError::UserError(err))
            }
        }

        (users, non_fatal_errors)
    }

    pub fn create_from_file_with_res(path: &str) -> Result<Self, DataCreationError> {
        let mut users = UserCollection::default();
        let unprocessed_user_line = UnprocessedUserLine::import_data_from_file_with_res(path)?;

        for line in unprocessed_user_line {
            users.push_with_res(User::try_from(line)?)?;
        }

        Ok(users)
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
    // TODO: This function will be deprecated in the future, as it seems better to just create a
    // subset and calculate from it.
    #[allow(deprecated)]
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

    #[allow(deprecated)]
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

    pub fn data(&self) -> &HashMap<usize, User> {
        &self.map
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum DataCreationError {
    #[error("Input data is invalid.")]
    InvalidInputError(#[from] InvalidInputError),

    #[error("UserError")]
    UserError(#[from] UserError),

    #[error("Input is not readable or accessible")]
    DataReadError(#[from] DataReadError),
}

#[derive(Error, Debug)]
pub enum DbReadError {
    #[error("fs error")]
    FSError(#[from] std::io::Error),

    #[error("json error")]
    JSONError(#[from] serde_json::Error),
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn test_user_count_on_file_with_res() {
        let users =
            UserCollection::create_from_file_with_res("data/dummy-data/spam.jsonl").unwrap();
        assert_eq!(users.user_count(), 2);
    }

    #[test]
    pub fn test_error_on_nonexisting_file() {
        assert_eq!(
            UserCollection::create_from_file_with_res("no-data-here"),
            Err(DataCreationError::DataReadError(
                DataReadError::InvalidDataPathError {
                    path: "no-data-here".to_string()
                }
            ))
        )
    }

    #[test]
    pub fn test_error_on_nonexisting_dir() {
        assert_eq!(
            UserCollection::create_from_dir_with_res("no-data-here"),
            Err(DataCreationError::DataReadError(
                DataReadError::InvalidDataPathError {
                    path: "no-data-here".to_string()
                }
            ))
        )
    }

    #[test]
    pub fn test_error_on_invalid_json_with_error_collect() {
        let users = UserCollection::create_from_file_and_collect_non_fatal_errors(
            "data/invalid-data/data.jsonl",
        );
        match users {
            Err(DataCreationError::DataReadError(DataReadError::InvalidJsonlError(..))) => (),
            Err(_) => panic!(),
            Ok(_) => panic!(),
        }
    }

    #[test]
    pub fn test_spam_score_collision_with_error_collect() {
        let users = UserCollection::create_from_file_and_collect_non_fatal_errors(
            "data/invalid-data/collision_data.jsonl",
        );

        assert!(users.is_ok());

        // assert that errors is of length one and contains a SpamCollisionError.
        let (data, errors) = users.unwrap();
        assert_eq!(errors.len(), 1);
        match errors[0] {
            DataCreationError::UserError(UserError::SpamScoreCollision { .. }) => (),
            _ => panic!(),
        }

        // check that the data contains one user.
        assert_eq!(data.user_count(), 1);
    }

    #[test]
    pub fn test_error_on_nonexisting_dir_with_error_collect() {
        assert_eq!(
            UserCollection::create_from_dir_and_collect_non_fatal_errors("no-data-here"),
            Err(DataCreationError::DataReadError(
                DataReadError::InvalidDataPathError {
                    path: "no-data-here".to_string()
                }
            ))
        )
    }

    #[test]
    pub fn test_error_on_invalid_jsonl_data_on_file() {
        let users = UserCollection::create_from_file_with_res("data/invalid-data/data.jsonl");
        match users {
            Err(DataCreationError::DataReadError(DataReadError::InvalidJsonlError(..))) => (),
            Err(_) => panic!(),
            Ok(_) => panic!(),
        }
    }

    #[test]
    pub fn test_error_on_spam_score_collision() {
        let users =
            UserCollection::create_from_file_with_res("data/invalid-data/collision_data.jsonl");
        match users {
            Err(DataCreationError::UserError(UserError::SpamScoreCollision { .. })) => (),
            Err(_) => panic!(),
            Ok(_) => panic!(),
        }
    }

    #[test]
    pub fn test_error_on_invalid_fid() {
        let users =
            UserCollection::create_from_file_with_res("data/invalid-data/invalid_spamscore.jsonl");
        match users {
            Err(DataCreationError::InvalidInputError(InvalidInputError::SpamScoreError {
                ..
            })) => (),
            Err(_) => panic!(),
            Ok(_) => panic!(),
        }
    }

    #[test]
    pub fn test_user_count_on_dir_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data/").unwrap();
        assert_eq!(users.user_count(), 2);
    }

    #[test]
    pub fn test_user_count_at_date_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data/").unwrap();
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
    fn test_spam_distribution_for_users_created_at_or_after_date_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let date = NaiveDate::from_ymd_opt(2025, 1, 23).unwrap();
        let closure = |user: &User| user.created_at_or_after_date_with_opt(date).unwrap();

        assert_eq!(
            users.spam_score_distribution_for_subset(closure),
            Some([0.0, 0.0, 1.0])
        );
    }

    #[test]
    fn test_apply_filter_for_one_fid_with_new() {
        let mut users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let closure = |user: &User| user.fid() == 2;
        users.apply_filter(closure);
        assert_eq!(
            users.current_spam_score_distribution(),
            Some([0.0, 0.0, 1.0])
        )
    }

    #[test]
    fn test_none_for_filtered_spam_distribution_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data").unwrap();
        let closure = |user: &User| user.fid() == 3;

        assert_eq!(users.spam_score_distribution_for_subset(closure), None);
    }
}
