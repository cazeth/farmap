#![allow(unused)]
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct CastMeta {
    cast_fid: u64,
    cast_time: NaiveDate,
    cast_type: CastType,
}
impl CastMeta {
    pub fn new(cast_time: NaiveDate, cast_type: CastType, cast_fid: u64) -> Self {
        Self {
            cast_time,
            cast_type,
            cast_fid,
        }
    }

    pub fn fid(&self) -> u64 {
        self.cast_fid
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Copy, Clone)]
pub enum CastType {
    CAST,
}

impl TryFrom<&str> for CastType {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "CAST" => Ok(Self::CAST),
            _ => Err("invalid string provided".to_string()),
        }
    }
}
