use thiserror::Error;

#[derive(Error, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum UserError {
    #[error(
        "User Value collides with existing user value. A User cannot contain colliding user values"
    )]
    CollisionError,
}
