use crate::dated::Dated;
use crate::user_value::AnyUserValue;
use crate::user_value::UserValueSeal;
use crate::UserValue;
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

impl UserValueSeal for Dated<CastType> {}

impl UserValue for Dated<CastType> {
    fn as_any_user_value(&self) -> AnyUserValue {
        AnyUserValue::DatedCastType(*self)
    }

    fn into_any_user_value(self) -> AnyUserValue {
        AnyUserValue::DatedCastType(self)
    }

    fn from_any_user_value(any_user_value: AnyUserValue) -> Option<Self> {
        match any_user_value {
            AnyUserValue::DatedCastType(x) => Some(x),
            _ => None,
        }
    }

    fn from_any_user_value_ref(any_user_value: &AnyUserValue) -> Option<&Self> {
        match any_user_value {
            AnyUserValue::DatedCastType(x) => Some(x),
            _ => None,
        }
    }
}
