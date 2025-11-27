#![allow(refining_impl_trait)]
use crate::dated::Dated;
use crate::native_user_value::AnyNativeUserValue;
use crate::native_user_value::NativeUserValueSeal;
use crate::utils::distribution_from_counts;
use crate::Collidable;
use crate::NativeUserValue;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SpamScore {
    Zero,
    One,
    Two,
}

pub type SpamScoreWithSourceCommit = (SpamScore, CommitHash);
pub type SpamRecord = (SpamScore, NaiveDate);
pub type DatedSpamScoreCount = Dated<SpamScoreCount>;
pub type DatedSpamScoreDistribution = Dated<SpamScoreDistribution>;
pub type DatedSpamUpdate = Dated<SpamUpdate>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SpamScoreCount {
    nonspam: u64,
    maybe: u64,
    spam: u64,
}

impl SpamScoreCount {
    pub fn new(spam_count: u64, maybe_count: u64, nonspam_count: u64) -> Self {
        Self {
            spam: spam_count,
            maybe: maybe_count,
            nonspam: nonspam_count,
        }
    }

    pub fn add(&mut self, score: SpamScore) {
        match score {
            SpamScore::Zero => self.spam += 1,
            SpamScore::One => self.maybe += 1,
            SpamScore::Two => self.nonspam += 1,
        }
    }

    pub fn total(&self) -> u64 {
        self.spam + self.maybe + self.nonspam
    }

    pub fn spam(&self) -> u64 {
        self.spam
    }

    pub fn maybe_spam(&self) -> u64 {
        self.maybe
    }

    pub fn non_spam(&self) -> u64 {
        self.nonspam
    }
}

impl From<[u64; 3]> for SpamScoreCount {
    fn from(value: [u64; 3]) -> Self {
        Self {
            spam: value[0],
            maybe: value[1],
            nonspam: value[2],
        }
    }
}

impl From<SpamScoreCount> for [u64; 3] {
    fn from(value: SpamScoreCount) -> Self {
        [value.spam, value.maybe, value.nonspam]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpamScoreDistribution {
    nonspam: f32,
    maybe: f32,
    spam: f32,
}

impl SpamScoreDistribution {
    pub fn spam(&self) -> f32 {
        self.spam
    }

    pub fn maybe_spam(&self) -> f32 {
        self.maybe
    }

    pub fn non_spam(&self) -> f32 {
        self.nonspam
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct CommitHash(u32);

impl TryFrom<String> for CommitHash {
    type Error = InvalidHashError;

    fn try_from(full_commit_value: String) -> Result<Self, Self::Error> {
        if full_commit_value.len() != 40 {
            return Err(InvalidHashError(full_commit_value));
        };

        let shortened_commit = full_commit_value.chars().take(4).collect::<String>();
        let result = u32::from_str_radix(&shortened_commit, 16)
            .map_err(|_| InvalidHashError(full_commit_value))?;
        Ok(CommitHash(result))
    }
}

#[derive(Error, Debug)]
#[error("invalid hash: {0}")]
pub struct InvalidHashError(String);

impl TryFrom<usize> for SpamScore {
    type Error = SpamScoreError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            _ => Err(SpamScoreError::OutOfBoundsError { value }),
        }
    }
}

impl From<SpamScoreDistribution> for [f32; 3] {
    fn from(value: SpamScoreDistribution) -> Self {
        [value.spam, value.maybe, value.nonspam]
    }
}

impl TryFrom<SpamScoreCount> for SpamScoreDistribution {
    type Error = EmptyScoreCountError;
    fn try_from(value: SpamScoreCount) -> Result<Self, Self::Error> {
        let distribution =
            distribution_from_counts::<3>(&value.into()).ok_or(EmptyScoreCountError)?;
        Ok(SpamScoreDistribution {
            nonspam: distribution[2],
            maybe: distribution[1],
            spam: distribution[0],
        })
    }
}

#[derive(Debug, Error, PartialEq)]
#[non_exhaustive]
pub enum SpamScoreError {
    #[error("Tried to create a spam score but it was out of bounds: {value}")]
    OutOfBoundsError { value: usize },
}

#[derive(Debug, Error)]
#[error("trying to create a spam score distribution from an empty SpamScoreCount")]
pub struct EmptyScoreCountError;

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Copy, Hash)]
pub enum SpamUpdate {
    WithSourceCommit(SpamScoreWithSourceCommit),
    WithoutSourceCommit(SpamScore),
}

impl SpamUpdate {
    pub fn score(&self) -> SpamScore {
        match self {
            Self::WithSourceCommit(x) => x.0,
            Self::WithoutSourceCommit(x) => *x,
        }
    }
}

impl From<SpamScore> for SpamUpdate {
    fn from(value: SpamScore) -> Self {
        Self::WithoutSourceCommit(value)
    }
}

impl NativeUserValue for SpamUpdate {
    fn as_any_user_value(&self) -> AnyNativeUserValue {
        AnyNativeUserValue::SpamScore(self.score())
    }

    fn into_any_user_value(self) -> AnyNativeUserValue {
        AnyNativeUserValue::SpamScore(self.score())
    }

    fn from_any_user_value(any_user_value: AnyNativeUserValue) -> Option<Self> {
        match any_user_value {
            AnyNativeUserValue::SpamUpdate(x) => Some(x),
            _ => None,
        }
    }

    fn from_any_user_value_ref(any_user_value: &AnyNativeUserValue) -> Option<&Self> {
        match any_user_value {
            AnyNativeUserValue::SpamUpdate(x) => Some(x),
            _ => None,
        }
    }
}

impl NativeUserValueSeal for SpamUpdate {}

impl NativeUserValue for DatedSpamUpdate {
    fn as_any_user_value(&self) -> AnyNativeUserValue {
        AnyNativeUserValue::DatedSpamUpdate(*self)
    }

    fn into_any_user_value(self) -> AnyNativeUserValue {
        AnyNativeUserValue::DatedSpamUpdate(self)
    }

    fn from_any_user_value(any_user_value: AnyNativeUserValue) -> Option<Self> {
        match any_user_value {
            AnyNativeUserValue::DatedSpamUpdate(x) => Some(x),
            _ => None,
        }
    }

    fn from_any_user_value_ref(any_user_value: &AnyNativeUserValue) -> Option<&Self> {
        match any_user_value {
            AnyNativeUserValue::DatedSpamUpdate(x) => Some(x),
            _ => None,
        }
    }
}

impl From<(SpamScore, NaiveDate)> for DatedSpamUpdate {
    fn from(value: (SpamScore, NaiveDate)) -> Self {
        let spam_update: SpamUpdate = value.0.into();
        Dated::from(value.1, spam_update)
    }
}

impl NativeUserValueSeal for DatedSpamUpdate {}

impl Collidable for DatedSpamUpdate {
    fn is_collision(&self, other: &Self) -> bool {
        self.date() == other.date() && self.score() != other.score()
    }
}

#[derive(Error, Debug)]
#[error("trying to create a SpamEntries from an empty struct")]
pub struct EmptyEntriesError;

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn basic_spam_score_count() -> DatedSpamScoreCount {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        DatedSpamScoreCount::from(date, [100, 150, 200])
    }

    #[test]
    pub fn test_label_value_invalid_input() {
        assert!(SpamScore::try_from(0).is_ok());
        assert!(SpamScore::try_from(1).is_ok());
        assert!(SpamScore::try_from(2).is_ok());
        assert!(SpamScore::try_from(3).is_err());
        assert!(SpamScore::try_from(100).is_err());
    }

    #[test]
    pub fn basic_spam_score_count_test() {
        let count = basic_spam_score_count();
        assert_eq!(count.spam(), 100);
        assert_eq!(count.maybe_spam(), 150);
        assert_eq!(count.non_spam(), 200);
        assert_eq!(count.total(), 100 + 150 + 200);
    }
}
