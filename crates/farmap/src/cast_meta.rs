use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, Hash)]
pub struct CastMeta {
    cast_fid: u64,
    cast_date: NaiveDate,
    cast_type: CastType,
}
impl CastMeta {
    pub fn new(cast_date: NaiveDate, cast_type: CastType, cast_fid: u64) -> Self {
        Self {
            cast_date,
            cast_type,
            cast_fid,
        }
    }

    pub fn fid(&self) -> u64 {
        self.cast_fid
    }

    pub fn cast_date(&self) -> NaiveDate {
        self.cast_date
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Copy, Clone, Hash)]
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
