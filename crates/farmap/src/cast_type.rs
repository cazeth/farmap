use crate::dated::Dated;
use crate::native_user_value::AnyNativeUserValue;
use crate::native_user_value::NativeUserValueSeal;
use crate::NativeUserValue;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Copy, Clone, Hash)]
#[non_exhaustive]
pub enum CastType {
    CAST,
}

impl TryFrom<&str> for CastType {
    type Error = InvalidCastInputError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "CAST" => Ok(Self::CAST),
            _ => Err(InvalidCastInputError::InvalidInput),
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum InvalidCastInputError {
    #[error("tried to create CastType with invalid input")]
    InvalidInput,
}

impl NativeUserValueSeal for Dated<CastType> {}

impl NativeUserValue for Dated<CastType> {
    fn as_any_user_value(&self) -> AnyNativeUserValue {
        AnyNativeUserValue::DatedCastType(*self)
    }

    fn into_any_user_value(self) -> AnyNativeUserValue {
        AnyNativeUserValue::DatedCastType(self)
    }

    fn from_any_user_value(any_user_value: AnyNativeUserValue) -> Option<Self> {
        match any_user_value {
            AnyNativeUserValue::DatedCastType(x) => Some(x),
            _ => None,
        }
    }

    fn from_any_user_value_ref(any_user_value: &AnyNativeUserValue) -> Option<&Self> {
        match any_user_value {
            AnyNativeUserValue::DatedCastType(x) => Some(x),
            _ => None,
        }
    }
}
