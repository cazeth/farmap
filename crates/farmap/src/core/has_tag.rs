/// Associate a tag with a value. It is typically used to add metadata to  [`UserValue`]. A tag can be many things, such as a source that the user value came from, a date associated with the data, or a fid. It is typically implemented on data where one wishes to relate one datapoint with another but where the inner type is still meaningful with the tag and where someone might wish to unwrap the data from the tag.
pub trait HasTag<T, Inner> {
    fn tag(&self) -> T;

    fn untag(self) -> (T, Inner);
}
