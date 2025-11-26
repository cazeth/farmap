#[derive(Debug, PartialEq, Clone, Hash)]
#[non_exhaustive]
pub enum CollectionError {
    UserValueCollisionError,
    DuplicateUserError,
}

impl std::error::Error for CollectionError {}

impl std::fmt::Display for CollectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserValueCollisionError => {
                write!(f, "Tried to add colliding value")
            }
            Self::DuplicateUserError => {
                write!(f, "User already exists in collection")
            }
        }
    }
}
