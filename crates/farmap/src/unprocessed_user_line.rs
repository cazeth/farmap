use crate::spam_score::DatedSpamUpdate;
use crate::spam_score::SpamScoreError;
use crate::Fid;
use crate::Fidded;
use crate::SpamScore;
use chrono::DateTime;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct UnprocessedUserLine {
    provider: usize,
    r#type: Type,
    label_type: String,
    label_value: usize,
    timestamp: usize,
}

impl UnprocessedUserLine {
    pub fn provider(&self) -> usize {
        self.provider
    }

    pub fn fid(&self) -> usize {
        self.r#type.fid as usize
    }

    pub fn label_value(&self) -> usize {
        self.label_value
    }

    pub fn timestamp(&self) -> usize {
        self.timestamp
    }

    pub fn date(&self) -> Result<NaiveDate, SpamDataParseError> {
        if let Some(date) = DateTime::from_timestamp(self.timestamp().try_into().unwrap(), 0) {
            Ok(date.date_naive())
        } else {
            Err(SpamDataParseError::DateError {
                timestamp: self.timestamp(),
            })
        }
    }
}

impl TryFrom<UnprocessedUserLine> for Fidded<DatedSpamUpdate> {
    type Error = SpamDataParseError;
    fn try_from(value: UnprocessedUserLine) -> Result<Self, Self::Error> {
        let fid = value.fid();
        let date = value.date()?;
        value.label_value();
        let spam_score = SpamScore::try_from(value.label_value())?;

        let dated_spam_update = DatedSpamUpdate::from(date, spam_score);
        let fid = Fid::from(fid);
        let fidded: Fidded<DatedSpamUpdate> = Fidded::from((dated_spam_update, fid));
        Ok(fidded)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct Type {
    fid: u64,
    target: String,
}

#[derive(Error, Debug, PartialEq)]
pub enum SpamDataParseError {
    #[error(transparent)]
    SpamScoreError(#[from] SpamScoreError),
    #[error("Timestamp was {0}, which is invalid.", . timestamp)]
    DateError { timestamp: usize },
}
