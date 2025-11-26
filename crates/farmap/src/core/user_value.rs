use super::any_user_value::AnyUserValue;

/// A representation of a data point related to a user.
///
/// Notably, a UserValue is not required to return a fid value. This is because UserValues are
/// typically stored in a [`UserStore`], which holds all UserValues for a Fid while storing the fid
/// value only once for each once instead of in every UserValue. But in some
/// situations, such as when you fetch data, you likely want to include which Fid the data belongs
/// to. That can be achieved by implementing the HasTag<Fid>. An iterator over values that
/// are UserValue and HasTag<Fid> can be passed to a [`UserCollection`], which will store the data
/// more efficiently.
///
/// A single UserStore can store many kinds of UserValues at the same time. This requires also
/// implementing AnyUserValue to state all the valid UserValues that can be stored in the same
/// UserStore. See [`AnyUserValue`] for more information.
pub trait UserValue<T: AnyUserValue>: Sized {
    fn into_any(self) -> T;
    fn as_any(&self) -> T;
    fn from_any(list: T) -> Option<Self>;
    fn from_any_ref(list: &T) -> Option<&Self>;
}
