use crate::{user::InvalidInputError, utils::distribution_from_counts};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
    nonspam: u64,
    maybe: u64,
    spam: u64,
}

impl SpamScoreCount {
    pub fn new(date: NaiveDate, spam_count: u64, maybe_count: u64, nonspam_count: u64) -> Self {
        Self {
            date,
            nonspam: nonspam_count,
            maybe: maybe_count,
            spam: spam_count,
        }
    }

    pub fn date(&self) -> NaiveDate {
        self.date
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

    pub fn add(&mut self, score: &SpamScore) {
        match score {
            SpamScore::Zero => self.spam += 1,
            SpamScore::One => self.maybe += 1,
            SpamScore::Two => self.nonspam += 1,
        }
    }

    pub fn total(&self) -> u64 {
        self.spam + self.maybe + self.nonspam
    }

    pub fn distributions(&self) -> Option<[f32; 3]> {
        distribution_from_counts(&[self.spam, self.maybe, self.nonspam])
    }
}

#[derive(Serialize, Debug)]
pub struct SpamScoreDistribution {
    date: NaiveDate,
    nonspam: f64,
    maybe: f64,
    spam: f64,
}

impl SpamScoreDistribution {
    pub fn new(date: NaiveDate, spam: f64, maybe: f64, nonspam: f64) -> Result<Self, String> {
        let sum = spam + maybe + nonspam;
        if !(0.99..=1.01).contains(&sum) {
            Err("provided values are not a distribution".to_string())
        } else {
            Ok(Self {
                date,
                nonspam,
                maybe,
                spam,
            })
        }
    }

    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn spam(&self) -> f64 {
        self.spam
    }

    pub fn maybe_spam(&self) -> f64 {
        self.maybe
    }

    pub fn non_spam(&self) -> f64 {
        self.nonspam
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
