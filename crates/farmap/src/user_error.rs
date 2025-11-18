use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
#[non_exhaustive]
pub enum UserError {
    #[error(
        "User Value collides with existing user value. A User cannot contain colliding user values"
    )]
    CollisionError,
}
