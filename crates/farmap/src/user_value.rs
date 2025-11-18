use crate::cast_type::CastType;
use crate::dated::Dated;
use crate::follow_count::FollowCount;
use crate::spam_score::{DatedSpamUpdate, SpamScore, SpamUpdate};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

#[allow(private_bounds)]
pub trait UserValue:
    Serialize + DeserializeOwned + Clone + Debug + PartialEq + UserValueSeal
{
    fn into_any_user_value(self) -> AnyUserValue;

    fn as_any_user_value(&self) -> AnyUserValue;

    fn from_any_user_value(any_user_value: AnyUserValue) -> Option<Self>;

    fn from_any_user_value_ref(any_user_value: &AnyUserValue) -> Option<&Self>;
}

pub(crate) trait UserValueSeal {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[non_exhaustive]
pub enum AnyUserValue {
    DatedSpamUpdate(DatedSpamUpdate),
    SpamUpdate(SpamUpdate),
    SpamScore(SpamScore),
    DatedCastType(Dated<CastType>),
    FollowCount(FollowCount),
}

impl AnyUserValue {
    pub fn specify<T: UserValue>(self) -> Option<T> {
        T::from_any_user_value(self)
    }

    pub fn specify_ref<T: UserValue>(&self) -> Option<&T> {
        T::from_any_user_value_ref(self)
    }
}
