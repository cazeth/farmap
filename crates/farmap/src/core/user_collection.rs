use super::user_value::UserValue;
use super::AnyUserValue;
use super::CollectionError;
use super::Fid;
use super::HasTag;
use super::UserStore;
use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;

/// A representation of one or several users.
///
/// This is intended to be the main struct to store user data.
///
/// The type parameter T refers to the [`AnyUserValue`] that all the users in the collection
/// implement. All users in the collection must be generic over the same [`AnyUserValue`].
///
pub struct UserCollection<T: AnyUserValue> {
    map: HashMap<Fid, UserStore<T>>,
}

#[expect(unused)]
impl<T: AnyUserValue> UserCollection<T> {
    pub fn add_user_value_iter<S: UserValue<T>, F: IntoIterator<Item = impl HasTag<Fid, S>>>(
        &mut self,
        values: F,
    ) -> Option<Vec<CollectionError>> {
        let mut errors: Option<Vec<CollectionError>> = None;
        for value in values {
            if let Some(user) = self.user_mut(value.tag()) {
                user.add_user_value(value.untag().1)
            } else {
                let fid = value.tag();
                let mut user = UserStore::from(fid);
                user.add_user_value(value.untag().1);
                self.add_user(user).expect("new user cannot collide");
            }
        }
        errors
    }

    pub fn add_user(&mut self, user: UserStore<T>) -> Result<(), CollectionError> {
        if let Vacant(entry) = self.map.entry(user.fid()) {
            entry.insert(user);
            Ok(())
        } else {
            Err(CollectionError::DuplicateUserError)
        }
    }

    pub fn user_mut(&mut self, fid: Fid) -> Option<&mut UserStore<T>> {
        self.map.get_mut(&fid)
    }
}
