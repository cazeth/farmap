use std::fmt::Debug;
use std::num::TryFromIntError;

use serde::{Deserialize, Serialize};
#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd)]
pub struct Fid(u64);

impl std::fmt::Display for Fid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for Fid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for Fid {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<usize> for Fid {
    fn from(value: usize) -> Self {
        let value = value as u64;
        Self(value)
    }
}

impl From<u32> for Fid {
    fn from(value: u32) -> Self {
        let value = value as u64;
        Self(value)
    }
}

impl TryFrom<i32> for Fid {
    type Error = TryFromIntError;

    fn try_from(value: i32) -> Result<Self, TryFromIntError> {
        let value: Result<u64, TryFromIntError> = value.try_into();
        Ok(Self(value?))
    }
}

impl From<Fid> for usize {
    fn from(value: Fid) -> Self {
        value.0 as usize
    }
}

impl From<Fid> for u64 {
    fn from(value: Fid) -> Self {
        value.0
    }
}
