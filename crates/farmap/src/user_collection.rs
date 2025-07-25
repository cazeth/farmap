use crate::fetch::DataReadError;
use crate::spam_score::SpamEntry;
use crate::spam_score::SpamScore;
use crate::user::InvalidInputError;
use crate::user::User;
use crate::user::UserError;
use crate::utils::distribution_from_counts;
use crate::UnprocessedUserLine;
use chrono::NaiveDate;
use serde::Deserialize;
use serde::Serialize;
use std::collections::hash_map::Entry::Vacant;
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

pub type CreateResult = Result<(UserCollection, Vec<DataCreationError>), DataCreationError>;

impl UserCollection {
    /// add a user to the collection. If the fid already exists, the label is updated.
    /// This method may fail if the user is considered invalid in UserCollection because of
    /// SpamScoreCollision.
    #[deprecated(since = "0.9.1", note = "use add_user_instead")]
    #[allow(deprecated)]
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
        Some(user.latest_spam_record()?.0)
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
            .filter(|(_, user)| user.spam_score_at_date_with_owned(&date).is_some())
            .count()
    }

    /// A data importer that keeps running in case of nonfatal errors.
    /// Nonfatal errors are spam collision errors or invalid parameter data. In case of such error
    /// the import continues to run and returns the errors in a vec alongside the return data.
    #[deprecated(note = "use local_spam_label_importer instead")]
    #[allow(deprecated)]
    pub fn create_from_dir_and_collect_non_fatal_errors(dir: &str) -> CreateResult {
        // these errors are considered fatal for now.
        let lines = UnprocessedUserLine::import_data_from_dir_with_res(dir)?;

        Ok(UserCollection::create_from_unprocessed_user_lines_and_collect_non_fatal_errors(lines))
    }

    /// Like create_from_dir ... but for a single file.
    #[deprecated(note = "use local_spam_label_importer instead")]
    #[allow(deprecated)]
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

    #[deprecated(
        since = "TBD",
        note = "create a User and add the user to the collection instead"
    )]
    pub fn push_unprocessed_user_line(
        &mut self,
        line: UnprocessedUserLine,
    ) -> Result<(), Box<dyn Error>> {
        let new_user = User::try_from(line)?;
        self.add_user(new_user)?;
        Ok(())
    }

    pub fn create_from_unprocessed_user_lines_and_collect_non_fatal_errors(
        lines: Vec<UnprocessedUserLine>,
    ) -> (UserCollection, Vec<DataCreationError>) {
        let mut users = UserCollection::default();
        let mut non_fatal_errors: Vec<DataCreationError> = Vec::new();

        for line in lines {
            let fid = line.fid();
            let spam_entry: SpamEntry = match SpamEntry::try_from(line) {
                Ok(spam_entry) => spam_entry,
                Err(err) => {
                    non_fatal_errors.push(DataCreationError::InvalidInputError(err));
                    continue;
                }
            };

            if let Some(user) = users.user_mut(fid) {
                if user.add_spam_record(spam_entry.record()).is_err() {
                    non_fatal_errors.push(DataCreationError::UserError(
                        UserError::SpamScoreCollision {
                            fid,
                            date: spam_entry.date(),
                            old_spam_score: user
                                .spam_score_at_date_with_owned(&spam_entry.date())
                                .unwrap(),
                            new_spam_score: spam_entry.score(),
                        },
                    ))
                };
            } else {
                let mut new_user = User::new_without_labels(fid);
                new_user
                    .add_spam_record(spam_entry.record())
                    .expect("adding spam entry should not fail for new user");
                users
                    .add_user(new_user)
                    .expect("adding user should not fail for if user doesn't exist in collection");
            }
        }

        (users, non_fatal_errors)
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
            .filter_map(|(_, user)| user.spam_score_at_date_with_owned(&date))
        {
            match spam_score {
                SpamScore::Zero => counts[0] += 1,
                SpamScore::One => counts[1] += 1,
                SpamScore::Two => counts[2] += 1,
            }
        }

        distribution_from_counts(&counts)
    }

    #[deprecated(note = "prefer using the equivalent functionality in subset instead")]
    pub fn current_spam_score_distribution(&self) -> Option<[f32; 3]> {
        let mut counts = [0; 3];
        for (_, user) in self.map.iter() {
            match user.latest_spam_record()?.0 {
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

    pub fn add_user(&mut self, user: User) -> Result<(), DuplicateUserError> {
        let fid = user.fid();
        if let Vacant(v) = self.map.entry(fid) {
            v.insert(user);
            Ok(())
        } else {
            Err(DuplicateUserError)
        }
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

    #[error("DuplicateUserError")]
    DuplicateUserError(#[from] DuplicateUserError),
}

#[derive(Error, Debug, PartialEq)]
#[error("user already exists in collection")]
pub struct DuplicateUserError;

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
    use chrono::NaiveDate;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    pub fn test_user_count_on_dir_with_new() {
        let db_path = PathBuf::from("data/dummy-data_db.json");
        let users = UserCollection::create_from_db(&db_path).unwrap();

        assert_eq!(users.user_count(), 2);
    }

    #[test]
    pub fn test_user_count_at_date_with_new() {
        let db_path = PathBuf::from("data/dummy-data_db.json");
        let users = UserCollection::create_from_db(&db_path).unwrap();
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
    fn serialize() {
        let mut collection = UserCollection::default();
        let record = (
            SpamScore::Zero,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        );
        let mut user = User::new_without_labels(1);
        user.add_spam_record(record).unwrap();
        let record = (SpamScore::Two, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        user.add_spam_record(record).unwrap();
        collection.add_user(user).unwrap();
        let json = json!(collection);
        let expected_json = r#"{"map":{"1":{"cast_records":null,"entries":{"entries":[{"WithoutSourceCommit":["Zero","2024-01-01"]},{"WithoutSourceCommit":["Two","2025-01-01"]}],"version":1},"fid":1,"latest_cast_record_check_date":null,"latest_reaction_time_update_date":null,"reaction_times":null}}}"#;
        assert_eq!(json.to_string(), expected_json);
    }
}
