use crate::cast_type::CastType;
use crate::dated::Dated;
use crate::try_from_user::TryFromUser;
use crate::try_from_user_set::TryFromUserSet;
use crate::Fid;
use crate::UserCollectionWithNativeUserValue;
use crate::UserStoreWithNativeUserValue;
use crate::UserWithCastData;
use crate::{UserSet, UsersSubset};
use thiserror::Error;

/// A set of users that contain at least one [`CastType`].
///
/// You can typically create this struct via [`TryFrom<UserCollection>`] or [`TryFromUserSet`].
/// Creation of the set is fallible since the set is not allowed to be empty.
pub struct SetWithCastData<'a> {
    set: UsersSubset<'a>,
}

impl<'a> SetWithCastData<'a> {
    /// The total casts averaged over the users in the set.
    pub fn average_total_casts(&self) -> f64 {
        let sum: usize = self
            .set
            .iter()
            .map(|x| x.user_values_of_kind::<Dated<CastType>>().len())
            .sum();
        sum as f64 / self.set.user_count() as f64
    }
}

impl<'a> IntoIterator for SetWithCastData<'a> {
    type Item = UserWithCastData<'a>;

    type IntoIter = std::iter::Map<
        std::collections::hash_map::IntoValues<Fid, &'a UserStoreWithNativeUserValue>,
        fn(&'a UserStoreWithNativeUserValue) -> UserWithCastData<'a>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.set
            .into_iter()
            .map(|user| UserWithCastData::try_from(user).unwrap())
    }
}

#[allow(refining_impl_trait)]
impl<'a> UserSet<'a> for SetWithCastData<'a> {
    fn user_count(&self) -> usize {
        self.set.user_count()
    }

    fn user(&'a self, fid: usize) -> Option<UserWithCastData<'a>> {
        self.set
            .user(fid)
            .map(|user| UserWithCastData::try_from(user).unwrap())
    }
}

impl<'a, T: UserSet<'a>> TryFromUserSet<'a, T> for SetWithCastData<'a> {
    type Error = SetWithCastDataError;
    fn try_from_set(value: T) -> Result<Self, Self::Error> {
        let results: Vec<UserWithCastData> = value
            .into_iter()
            .flat_map(<UserWithCastData as TryFromUser<_>>::try_from_user)
            .collect();
        if results.is_empty() {
            Err(SetWithCastDataError::EmptySetError)
        } else {
            let mut set = UsersSubset::default();
            for user in results {
                set.add_user(user);
            }
            Ok(Self { set })
        }
    }
}

impl<'a> TryFrom<&'a UserCollectionWithNativeUserValue> for SetWithCastData<'a> {
    type Error = SetWithCastDataError;
    fn try_from(collection: &'a UserCollectionWithNativeUserValue) -> Result<Self, Self::Error> {
        let set = UsersSubset::from_filter(collection, |user| user.has::<Dated<CastType>>());
        if set.user_count() != 0 {
            Ok(Self { set })
        } else {
            dbg!(&set);
            Err(SetWithCastDataError::EmptySetError)
        }
    }
}

/// This error indicates that the user tried to create a set that would be empty, which is not
/// allowed.
#[derive(Error, Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[non_exhaustive]
pub enum SetWithCastDataError {
    #[error("Tried to create a set that would be empty")]
    EmptySetError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_collection::UserCollectionWithNativeUserValue;
    use chrono::NaiveDate;

    fn same_date_cast_iter(count: usize, date: NaiveDate) -> impl Iterator<Item = Dated<CastType>> {
        let dated_cast_type: Dated<CastType> = (CastType::CAST, date).into();
        vec![dated_cast_type].into_iter().cycle().take(count)
    }

    #[allow(unused)]
    #[track_caller]
    fn create_valid_cast_user_set(
        collection: &UserCollectionWithNativeUserValue,
    ) -> SetWithCastData {
        collection.try_into().unwrap()
    }

    #[track_caller]
    fn check_err_on_invalid_cast_user_set(collection: &UserCollectionWithNativeUserValue) {
        let err = SetWithCastData::try_from(collection);
        if let Err(err) = err {
            assert_eq!(err, SetWithCastDataError::EmptySetError);
        } else {
            panic!("invalid user set should return error");
        }
    }

    mod test_try_from_collection {
        use super::*;
        use crate::user_collection::tests::{dummy_data, new_collection_from_user_value_iter};

        use super::check_err_on_invalid_cast_user_set;

        #[test]
        fn test_err_on_try_from_with_no_cast_data() {
            let collection = dummy_data();
            check_err_on_invalid_cast_user_set(&collection);
        }

        #[test]
        fn test_valid_create() {
            let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

            let collection = new_collection_from_user_value_iter(same_date_cast_iter(10, date));
            create_valid_cast_user_set(&collection);
        }
    }
}
