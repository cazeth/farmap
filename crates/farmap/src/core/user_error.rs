#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum UserError {
    CollisionError,
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "User Value collides with existing user value. A User cannot contain colliding user values")
    }
}

impl std::error::Error for UserError {}
