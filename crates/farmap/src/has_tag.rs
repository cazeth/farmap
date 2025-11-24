//! Associate a tag with a user value. A tag can be, many things, such as a source that the user
//! value came from, a date associated with the data, or a fid.

pub trait HasTag<T, Inner> {
    fn tag(&self) -> T;

    fn untag(self) -> (T, Inner);
}
