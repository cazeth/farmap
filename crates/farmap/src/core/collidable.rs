/// Check for conflicting values, i.e. values that are contradictory in some way.
pub trait Collidable {
    fn is_collision(&self, other: &Self) -> bool;
}

impl<T> Collidable for &T
where
    T: Collidable,
{
    fn is_collision(&self, other: &Self) -> bool {
        (*self).is_collision(other)
    }
}

impl<T> Collidable for &mut T
where
    T: Collidable,
{
    fn is_collision(&self, other: &Self) -> bool {
        Collidable::is_collision(&self, &other)
    }
}
