use super::AnyUserValue;
use super::Collidable;
use super::Fid;
use super::UserError;
use super::UserValue;

/// The main struct for storing user data.
///
/// An instance of UserStore is always related to a collection of potential user values that it can store simultaneously. This is expressed through the AnyUserValue, which connects this Struct with [`UserValue`].
/// This struct is generic over [`AnyUserValue`], which is that is the main data type that this struct store and what determines which kind [`UserValue`] can be stored. [`AnyUserValue`] is a type that can hold all the types that a UserStore can store.
///
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct UserStore<T: AnyUserValue> {
    fid: Fid,
    values: Vec<T>,
}

impl<T: AnyUserValue> UserStore<T> {
    pub fn has<S: UserValue<T>>(&self) -> bool {
        self.values.iter().any(|x| S::from_any_ref(x).is_some())
    }

    pub fn try_add_user_value<S: UserValue<T> + Collidable>(
        &mut self,
        new: S,
    ) -> Result<(), UserError> {
        if self
            .user_values_of_kind::<S>()
            .all(|x| !Collidable::is_collision(x, &new))
        {
            self.add_user_value(new);
            Ok(())
        } else {
            Err(UserError::CollisionError)
        }
    }

    pub fn add_user_value<S: UserValue<T>>(&mut self, new: S) {
        self.values.push(new.into_any());
    }

    pub fn user_values_of_kind<'a, S: UserValue<T> + 'a>(&'a self) -> impl Iterator<Item = &'a S> {
        self.values.iter().flat_map(|x| x.specify_ref::<S>())
    }

    pub(crate) fn from_generic_user_values(
        fid: impl Into<Fid>,
        values: impl IntoIterator<Item = T>,
    ) -> Self {
        let fid = fid.into();
        let values: Vec<T> = values.into_iter().collect();
        Self { fid, values }
    }

    pub fn fid(&self) -> Fid {
        self.fid
    }

    pub fn all_user_values<'a>(&'a self) -> impl Iterator<Item = &'a T> {
        self.values.iter()
    }
}

impl<T: AnyUserValue> From<Fid> for UserStore<T> {
    fn from(value: Fid) -> Self {
        Self {
            fid: value,
            values: Vec::new(),
        }
    }
}
