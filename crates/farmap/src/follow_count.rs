use crate::native_user_value::AnyNativeUserValue;
use crate::NativeUserValue;

use crate::native_user_value::NativeUserValueSeal;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
pub struct FollowCount(u64);

impl NativeUserValueSeal for FollowCount {}

impl NativeUserValue for FollowCount {
    fn as_any_user_value(&self) -> AnyNativeUserValue {
        AnyNativeUserValue::FollowCount(*self)
    }

    fn into_any_user_value(self) -> AnyNativeUserValue {
        AnyNativeUserValue::FollowCount(self)
    }

    fn from_any_user_value(any_user_value: AnyNativeUserValue) -> Option<Self> {
        match any_user_value {
            AnyNativeUserValue::FollowCount(x) => Some(x),
            _ => None,
        }
    }

    fn from_any_user_value_ref(any_user_value: &AnyNativeUserValue) -> Option<&Self> {
        match any_user_value {
            AnyNativeUserValue::FollowCount(x) => Some(x),
            _ => None,
        }
    }
}
