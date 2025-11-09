use crate::UserSet;

/// Falllibly create one kind of [`UserSet`] from another.
///
/// This is trait is different to the generic TryFrom because both sets involved likely are
/// [`UserSet`], which causes conflicts with the generic TryFrom trait.
pub trait TryFromUserSet<'a, T: UserSet<'a>>
where
    Self: Sized,
{
    type Error;
    fn try_from_set(value: T) -> Result<Self, Self::Error>;
}
