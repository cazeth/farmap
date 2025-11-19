use crate::fetch::DataReadError;
use crate::has_tag::HasTag;
use crate::user::User;
use crate::Fid;
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
        values: impl IntoIterator<Item = impl HasTag<Fid>>,
    ) -> Option<Vec<CollectionError>> {
        let mut errors: Option<Vec<CollectionError>> = None;
        for value in values {
            if let Some(user) = self.user_mut(value.tag()) {
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
                let mut user = User::new(value.tag());
                user.add_user_value(value.untag().0)
                    .expect("new user cannot collide");
                self.add_user(user).expect("new user cannot collide");
            }
        }
        errors
    }

    pub fn user_mut(&mut self, fid: impl Into<Fid>) -> Option<&mut User> {
        let fid: Fid = fid.into();
        let usize_fid: usize = fid.into();
        self.map.get_mut(&usize_fid)
    }

    #[allow(unused)]
    pub(crate) fn user_mut_unchecked(&mut self, fid: impl Into<Fid>) -> &mut User {
        let fid: Fid = fid.into();
        let usize_fid: usize = fid.into();
        self.map
            .get_mut(&usize_fid)
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
        let collection = serde_json::from_str(&std::fs::read_to_string(db)?)?;

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

    pub fn add_user(&mut self, user: User) -> Result<(), CollectionError> {
        let fid = user.fid();
        if let Vacant(v) = self.map.entry(fid) {
            v.insert(user);
            Ok(())
        } else {
            Err(CollectionError::DuplicateUserError)
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
    InvalidInputError,

    #[error("Input is not readable or accessible")]
    DataReadError(#[from] DataReadError),

    #[error("DuplicateUserError")]
    DuplicateUserError,
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
    use crate::UserValue;
    use std::path::PathBuf;

    #[test]
    pub fn test_user_count_on_dir_with_new() {
        assert_eq!(dummy_data().user_count(), 2);
    }

    impl<T> HasTag<Fid> for TestUserValue<T>
    where
        T: UserValue,
    {
        fn tag(&self) -> Fid {
            self.fid
        }

        #[allow(refining_impl_trait)]
        fn untag(self) -> (T, Fid) {
            (self.value, self.fid)
        }
    }

    struct TestUserValue<T: UserValue> {
        pub value: T,
        pub fid: Fid,
    }

    #[track_caller]
    pub fn collection_from_fidded<T: HasTag<Fid>>(
        values: impl IntoIterator<Item = T>,
    ) -> UserCollection {
        let mut collection = UserCollection::default();
        collection.add_user_value_iter(values);
        collection
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
                fid: (n as u64 + 1).into(),
            }
        }));
        assert!(res.is_none()); // There is one entry per fid so there should be no collisions
        collection
    }

    pub fn empty_collection() -> UserCollection {
        UserCollection::default()
    }

    pub fn dummy_data() -> UserCollection {
        let db_path = PathBuf::from("data/dummy-data_db.json");
        UserCollection::create_from_db(&db_path).unwrap()
    }

    pub mod add_user {

        use super::*;

        #[track_caller]
        pub fn check_add_user(collection: &mut UserCollection, user: User) {
            collection.add_user(user).unwrap()
        }
    }

    mod user_count {
        use super::add_user::check_add_user;
        use crate::user::tests::create_new_user;

        use super::*;

        #[track_caller]
        fn check_user_count(collection: &UserCollection, n: usize) {
            assert_eq!(collection.user_count(), n)
        }

        #[test]
        fn test_empty_user_count() {
            let collection = empty_collection();
            check_user_count(&collection, 0);
        }

        #[test]
        fn test_non_empty_user_count() {
            let mut collection = empty_collection();
            check_add_user(&mut collection, create_new_user(1));
            check_add_user(&mut collection, create_new_user(2));
            check_user_count(&collection, 2);
        }
    }
}
