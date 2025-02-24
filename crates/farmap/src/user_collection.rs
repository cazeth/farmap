use crate::spam_score::SpamScore;
use crate::user::DataReadError;
use crate::user::InvalidInputError;
use crate::user::UnprocessedUserLine;
use crate::user::User;
use crate::user::UserError;
use crate::utils::distribution_from_counts;
use chrono::NaiveDate;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Default, Debug, PartialEq)]
pub struct UserCollection {
    map: HashMap<usize, User>,
}

type CreateResult = Result<(UserCollection, Vec<DataCreationError>), DataCreationError>;

impl UserCollection {
    /// add a user to the collection. If the fid already exists, the label is updated.
    #[deprecated(note = "use push_with_res instead")]
    #[doc(hidden)]
    #[allow(deprecated)]
    pub fn push(&mut self, user: User) -> bool {
        if let Some(existing_user) = self.map.get_mut(&user.fid()) {
            existing_user.update_user(user);
            false
        } else {
            self.map.insert(user.fid(), user);
            true
        }
    }

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

        // if errors occur while importing a particular line the parsing continues and collects the errors.
        Ok(UserCollection::create_from_unprocessed_user_lines_and_collect_non_fatal_errors(lines))
    }

    /// Like create_from_dir ... but for a single file.
    pub fn create_from_file_and_collect_non_fatal_errors(file: &str) -> CreateResult {
        // these errors are considered fatal for now.
        let lines = UnprocessedUserLine::import_data_from_file_with_res(file)?;

        // if errors occur while importing a particular line the parsing continues and collects the errors.
        Ok(UserCollection::create_from_unprocessed_user_lines_and_collect_non_fatal_errors(lines))
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

    #[deprecated(note = "use create_from_dir_with_res instead")]
    #[doc(hidden)]
    #[allow(deprecated)]
    pub fn create_from_dir(dir: &str) -> Self {
        let unprocessed_user_line = UnprocessedUserLine::import_data_from_dir(dir);
        let mut users = UserCollection::default();
        for line in unprocessed_user_line {
            users.push(User::try_from(line).unwrap());
        }
        users
    }

    #[deprecated(note = "use create_from_file_with_res_instead")]
    #[doc(hidden)]
    #[allow(deprecated)]
    pub fn create_from_file(path: &str) -> Self {
        let mut users = UserCollection::default();
        let unprocessed_user_line = UnprocessedUserLine::import_data_from_file(path);

        for line in unprocessed_user_line {
            users.push(User::try_from(line).unwrap());
        }

        users
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

    /// this test has been replaced with test_users_count_on_file_with_res and will be removed once
    /// fn create_from_file_is_removed.
    #[test]
    #[allow(deprecated)]
    pub fn test_user_count_on_file() {
        let users = UserCollection::create_from_file("data/dummy-data/spam.jsonl");
        assert_eq!(users.user_count(), 2);
        let users = UserCollection::create_from_dir("data/dummy-data/");
        assert_eq!(users.user_count(), 2);
    }

    #[test]
    pub fn test_user_count_on_dir_with_new() {
        let users = UserCollection::create_from_dir_with_res("data/dummy-data/").unwrap();
        assert_eq!(users.user_count(), 2);
    }

    #[test]
    #[allow(deprecated)]
    pub fn test_user_count_on_dir() {
        let users = UserCollection::create_from_dir("data/dummy-data/");
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
    #[allow(deprecated)]
    pub fn test_user_count_at_date() {
        let users = UserCollection::create_from_dir("data/dummy-data/");
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
        let closure = |user: &User| user.created_at_or_after_date(date);

        assert_eq!(
            users.spam_score_distribution_for_subset(closure),
            Some([0.0, 0.0, 1.0])
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_spam_distribution_for_users_created_at_or_after_date() {
        let users = UserCollection::create_from_dir("data/dummy-data");
        let date = NaiveDate::from_ymd_opt(2025, 1, 23).unwrap();
        let closure = |user: &User| user.created_at_or_after_date(date);

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
    #[allow(deprecated)]
    fn test_apply_filter_for_one_fid() {
        let mut users = UserCollection::create_from_dir("data/dummy-data");
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

    #[test]
    #[allow(deprecated)]
    fn test_none_for_filtered_spam_distribution() {
        let users = UserCollection::create_from_dir("data/dummy-data");
        let closure = |user: &User| user.fid() == 3;

        assert_eq!(users.spam_score_distribution_for_subset(closure), None);
    }
}
