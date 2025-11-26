use crate::cast_type::CastType;
use crate::core::AnyUserValue;
use crate::core::UserValue;
use crate::dated::Dated;
use crate::follow_count::FollowCount;
use crate::spam_score::{DatedSpamUpdate, SpamScore, SpamUpdate};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

#[allow(private_bounds)]
pub trait NativeUserValue:
    Serialize + DeserializeOwned + Clone + Debug + PartialEq + NativeUserValueSeal
{
    fn into_any_user_value(self) -> AnyNativeUserValue;

    fn as_any_user_value(&self) -> AnyNativeUserValue;

    fn from_any_user_value(any_user_value: AnyNativeUserValue) -> Option<Self>;

    fn from_any_user_value_ref(any_user_value: &AnyNativeUserValue) -> Option<&Self>;
}

pub(crate) trait NativeUserValueSeal {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[non_exhaustive]
pub enum AnyNativeUserValue {
    DatedSpamUpdate(DatedSpamUpdate),
    SpamUpdate(SpamUpdate),
    SpamScore(SpamScore),
    DatedCastType(Dated<CastType>),
    FollowCount(FollowCount),
}

impl AnyNativeUserValue {
    pub fn specify<T: NativeUserValue>(self) -> Option<T> {
        T::from_any_user_value(self)
    }

    pub fn specify_ref<T: NativeUserValue>(&self) -> Option<&T> {
        T::from_any_user_value_ref(self)
    }
}

impl AnyUserValue for AnyNativeUserValue {
    fn specify_ref<S: UserValue<AnyNativeUserValue>>(&self) -> Option<&S> {
        S::from_any_ref(self)
    }

    fn specify<S: UserValue<AnyNativeUserValue>>(self) -> Option<S> {
        S::from_any(self)
    }
}
