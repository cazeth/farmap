use super::user_value::UserValue;

/// A representation that can store different kinds of  [`UserValue`]s.
/// It can only be converted into one [`UserValue`] at the time, a that is carried out via the specify methods.
pub trait AnyUserValue: Sized {
    fn specify<S: UserValue<Self>>(self) -> Option<S>;
    fn specify_ref<S: UserValue<Self>>(&self) -> Option<&S>;
}
