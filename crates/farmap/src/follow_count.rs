use crate::user_value::AnyUserValue;
use crate::user_value::UserValueSeal;
use crate::UserValue;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub struct FollowCount(u64);

impl UserValueSeal for FollowCount {}

impl UserValue for FollowCount {
    fn as_any_user_value(&self) -> AnyUserValue {
        AnyUserValue::FollowCount(*self)
    }

    fn into_any_user_value(self) -> AnyUserValue {
        AnyUserValue::FollowCount(self)
    }

    fn from_any_user_value(any_user_value: AnyUserValue) -> Option<Self> {
        match any_user_value {
            AnyUserValue::FollowCount(x) => Some(x),
            _ => None,
        }
    }

    fn from_any_user_value_ref(any_user_value: &AnyUserValue) -> Option<&Self> {
        match any_user_value {
            AnyUserValue::FollowCount(x) => Some(x),
            _ => None,
        }
    }
}
