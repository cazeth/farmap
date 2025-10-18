use crate::spam_score::{DatedSpamEntry, DatedSpamUpdate, SpamScore, SpamUpdate};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

pub trait UserValue:
    Serialize + DeserializeOwned + Clone + Debug + PartialEq + UserValueSeal
{
    fn into_any_user_value(self) -> AnyUserValue;

    fn as_any_user_value(&self) -> AnyUserValue;

    fn from_any_user_value(any_user_value: AnyUserValue) -> Option<Self>;

    fn from_any_user_value_ref(any_user_value: &AnyUserValue) -> Option<&Self>;
}

pub trait UserValueSeal {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnyUserValue {
    DatedSpamEntry(DatedSpamEntry),
    DatedSpamUpdate(DatedSpamUpdate),
    SpamUpdate(SpamUpdate),
    SpamScore(SpamScore),
}

impl AnyUserValue {
    pub fn specify<T: UserValue>(self) -> Option<T> {
        T::from_any_user_value(self)
    }

    pub fn specify_ref<T: UserValue>(&self) -> Option<&T> {
        T::from_any_user_value_ref(self)
    }
}
