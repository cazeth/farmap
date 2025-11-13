use crate::fetch::DataReadError;
use crate::has_tag::HasTag;
use crate::spam_score::DatedSpamUpdate;
use crate::user::InvalidInputError;
use crate::user::User;
use crate::user::UserError;
use crate::UnprocessedUserLine;
use crate::UsersSubset;
use itertools::Itertools;
use log::warn;
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
    /// Returns a vec of all the collision errors, if there are any.
    pub fn add_user_value_iter(
        &mut self,
        values: impl IntoIterator<Item = impl HasTag<u64>>,
    ) -> Option<Vec<CollectionError>> {
        let mut errors: Option<Vec<CollectionError>> = None;
        for value in values {
            if let Some(user) = self.user_mut(value.tag() as usize) {
                let user_add_result = user
                    .add_user_value(value.untag().0)
                    .map_err(|_| CollectionError::UserValueCollisionError);
                if let Err(add_result) = user_add_result {
                    if let Some(errors) = &mut errors {
                        errors.push(add_result);
                    } else {
                        errors = Some(vec![add_result])
                    }
                }
            } else {
                let mut user = User::new_without_labels(value.tag() as usize);
                user.add_user_value(value.untag().0)
                    .expect("new user cannot collide");
                self.add_user(user).expect("new user cannot collide");
            }
        }
        errors
    }

    pub fn user_mut(&mut self, fid: usize) -> Option<&mut User> {
        self.map.get_mut(&fid)
    }

    pub(crate) fn user_mut_unchecked(&mut self, fid: usize) -> &mut User {
        self.map
            .get_mut(&fid)
            .expect("fid {fid} should exist in collection")
    }

    pub fn user(&self, fid: usize) -> Option<&User> {
        self.map.get(&fid)
    }

    pub fn user_count(&self) -> usize {
        self.map.len()
    }

    // the problem with this is that when the file does not exist the program will fail because
    // there isn't really a way for the caller to anticipate this...
    pub fn create_from_db(db: &Path) -> Result<Self, DbReadError> {
        let mut collection = serde_json::from_str(&std::fs::read_to_string(db)?)?;

        // this code refactors a user_collection to saves spam_entries as spam_updates.
        let fid_update_list: Vec<usize>;

        {
            let mut set = UsersSubset::from(&collection);
            set.filter(|user| {
                user.labels().is_some() && user.user_values_of_kind::<DatedSpamUpdate>().is_empty()
            });

            fid_update_list = set.iter().map(|user| user.fid()).collect_vec();
        }

        if !fid_update_list.is_empty() {
            warn!("discovered users in user collection with spam entries but no spam updates. Make sure to overwrite the database at the end of the program");
        }

        for fid in fid_update_list {
            let user = collection.user_mut_unchecked(fid);
            for spam_entry in user.all_spam_records_with_opt().unwrap().clone() {
                let spam_update = DatedSpamUpdate::from(spam_entry.1, spam_entry.0);
                user.add_user_value(spam_update).expect("should exist");
            }
        }

        Ok(collection)
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

#[derive(Error, Debug, PartialEq, Clone, Hash)]
#[non_exhaustive]
pub enum CollectionError {
    #[error("Tried to add colliding value")]
    UserValueCollisionError,

    #[error("user already exists in collection")]
    DuplicateUserError,
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
    use crate::user_with_spam_data::tests::create_user_with_m_spam_scores;
    use crate::UserValue;
    use chrono::NaiveDate;
    use std::path::PathBuf;

    #[test]
    pub fn test_user_count_on_dir_with_new() {
        assert_eq!(dummy_data().user_count(), 2);
    }

    impl<T> HasTag<u64> for TestUserValue<T>
    where
        T: UserValue,
    {
        fn tag(&self) -> u64 {
            self.fid
        }

        #[allow(refining_impl_trait)]
        fn untag(self) -> (T, u64) {
            (self.value, self.fid)
        }
    }

    struct TestUserValue<T: UserValue> {
        pub value: T,
        pub fid: u64,
    }

    pub fn new_collection_from_user_value_iter<T>(
        values: impl IntoIterator<Item = T>,
    ) -> UserCollection
    where
        T: UserValue,
    {
        let mut collection = UserCollection::default();
        let res = collection.add_user_value_iter(values.into_iter().enumerate().map(|(n, x)| {
            TestUserValue {
                value: x,
                fid: n as u64 + 1,
            }
        }));
        assert!(res.is_none()); // There is one entry per fid so there should be no collisions
        collection
    }

    pub fn empty_collection() -> UserCollection {
        UserCollection::default()
    }

    pub fn basic_single_user_test_collection_with_n_spam_updates(n: u64) -> UserCollection {
        let date = NaiveDate::parse_from_str("2020-1-1", "%Y-%m-%d").unwrap();
        let user = create_user_with_m_spam_scores(1, n, date);

        let mut user_collection = UserCollection::default();
        user_collection
            .add_user(user)
            .expect("only one user in collection - cannot collide");
        user_collection
    }

    pub fn dummy_data() -> UserCollection {
        let db_path = PathBuf::from("data/dummy-data_db.json");
        UserCollection::create_from_db(&db_path).unwrap()
    }
}
