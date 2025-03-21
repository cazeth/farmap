use crate::user::InvalidInputError;
use chrono::NaiveDate;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Hash, Eq)]
pub enum SpamScore {
    Zero,
    One,
    Two,
}

pub type SpamRecord = (SpamScore, NaiveDate);

impl TryFrom<usize> for SpamScore {
    type Error = InvalidInputError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            _ => Err(InvalidInputError::SpamScoreError { label: value }),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct SpamScoreCount {
    date: NaiveDate,
    #[serde(flatten)]
    counts: HashMap<SpamScore, u64>,
    #[serde(skip)]
    total: u64,
}

impl SpamScoreCount {
    pub fn new(date: NaiveDate, spam_count: u64, maybe_count: u64, nonspam_count: u64) -> Self {
        let mut map: HashMap<SpamScore, u64> = HashMap::new();
        use SpamScore::*;
        map.insert(Zero, spam_count);
        map.insert(One, maybe_count);
        map.insert(Two, nonspam_count);

        Self {
            date,
            counts: map,
            total: spam_count + maybe_count + nonspam_count,
        }
    }
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn spam(&self) -> u64 {
        *self.counts.get(&SpamScore::Zero).unwrap()
    }

    pub fn maybe_spam(&self) -> u64 {
        *self.counts.get(&SpamScore::One).unwrap()
    }

    pub fn non_spam(&self) -> u64 {
        *self.counts.get(&SpamScore::Two).unwrap()
    }

    pub fn total(&self) -> u64 {
        self.total
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn basic_spam_score_count() -> SpamScoreCount {
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        SpamScoreCount::new(date, 100, 150, 200)
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
