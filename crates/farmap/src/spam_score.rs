use crate::{user::InvalidInputError, utils::distribution_from_counts};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpamScore {
    Zero,
    One,
    Two,
}

pub type SpamRecord = (SpamScore, NaiveDate);

pub type SpamRecordWithSourceCommit = ((SpamScore, NaiveDate), CommitHash);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum SpamEntry {
    WithSourceCommit(SpamRecordWithSourceCommit),
    WithoutSourceCommit(SpamRecord),
}

impl SpamEntry {
    pub fn date(&self) -> NaiveDate {
        match self {
            Self::WithSourceCommit(x) => x.0 .1,
            Self::WithoutSourceCommit(x) => x.1,
        }
    }

    pub fn source(&self) -> Option<CommitHash> {
        todo!();
    }
}

impl From<SpamEntry> for SpamRecord {
    fn from(value: SpamEntry) -> Self {
        match value {
            SpamEntry::WithSourceCommit(x) => x.0,
            SpamEntry::WithoutSourceCommit(x) => x,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(try_from = "SerdeSpamEntries")]
#[serde(into = "SerdeSpamEntries")]
pub struct SpamEntries {
    entries: Vec<SpamEntry>,
}

impl SpamEntries {
    pub fn new(entry: SpamEntry) -> Self {
        let entries = vec![entry];
        Self { entries }
    }

    pub fn add_spam_entry(&mut self, entry: SpamEntry) -> Result<(), CollisionError> {
        let closest_element = self.entries.iter().find(|x| x.date() >= entry.date());
        let closest_position = self.entries.iter().position(|x| x.date() >= entry.date());

        match closest_element {
            Some(x) if *x == entry => Ok(()),
            None => {
                self.entries.push(entry);
                Ok(())
            }
            Some(x) if *x != entry && x.date() != entry.date() => {
                self.entries.insert(closest_position.unwrap(), entry);
                Ok(())
            }
            Some(x)
                if *x != entry
                    && x.date() == entry.date()
                    && x.score() == entry.score()
                    && x.source() != entry.source()
                    && entry.source().is_some() =>
            {
                self.entries.insert(closest_position.unwrap(), entry);
                Ok(())
            }
            Some(x)
                if *x != entry
                    && x.date() == entry.date()
                    && x.score() == entry.score()
                    && x.source() != entry.source()
                    && entry.source().is_none() =>
            {
                Ok(())
            }
            Some(x) if *x != entry && x.date() == entry.date() && x.score() != entry.score() => {
                Err(CollisionError {
                    date: entry.date(),
                    old_value: *x,
                    new_value: entry,
                })
            }
            Some(_) => {
                unreachable!()
            }
        }
    }

    pub fn earliest_spam_entry(&self) -> SpamEntry {
        *self.entries.first().unwrap()
    }

    pub fn last_spam_entry(&self) -> SpamEntry {
        *self.entries.last().unwrap()
    }

    pub fn spam_score_at_date(&self, date: NaiveDate) -> Option<SpamScore> {
        if date < self.earliest_spam_entry().date() {
            return None;
        };

        let pos = self
            .entries
            .iter()
            .rev()
            .position(|x| x.date() > date)
            .unwrap_or_else(|| self.entries.len() - 1);

        Some(self.entries.get(pos)?.score())
    }

    pub fn all_spam_entries(&self) -> &Vec<SpamEntry> {
        &self.entries
    }
}

#[derive(Deserialize, Serialize)]
pub struct SerdeSpamEntries {
    pub entries: Vec<SpamEntry>,
    pub version: usize,
}

impl TryFrom<SerdeSpamEntries> for SpamEntries {
    type Error = EmptyEntriesError;
    fn try_from(value: SerdeSpamEntries) -> Result<Self, Self::Error> {
        if !value.entries.is_empty() {
            Ok(SpamEntries {
                entries: value.entries,
            })
        } else {
            Err(EmptyEntriesError)
        }
    }
}

impl From<SpamEntries> for SerdeSpamEntries {
    fn from(value: SpamEntries) -> Self {
        Self {
            entries: value.entries,
            version: 1,
        }
    }
}

#[derive(Error, Debug)]
#[error("trying to create a SpamEntries from an empty struct")]
pub struct EmptyEntriesError;

#[derive(Error, Debug)]
#[error("Collision detected on date {date:?}: old value {old_value:?}, new value {new_value:?}")]
pub struct CollisionError {
    date: NaiveDate,
    old_value: SpamEntry,
    new_value: SpamEntry,
}

impl SpamEntry {
    pub fn score(&self) -> SpamScore {
        match self {
            Self::WithSourceCommit(x) => x.0 .0,
            Self::WithoutSourceCommit(x) => x.0,
        }
    }

    pub fn record(&self) -> SpamRecord {
        match self {
            Self::WithoutSourceCommit(x) => *x,
            Self::WithSourceCommit(x) => x.0,
        }
    }
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
    use serde_json::json;

    fn entry_without_source(year: i32, month: u32, day: u32, score: u8) -> SpamEntry {
        let typed_score = match score {
            0 => SpamScore::Zero,
            1 => SpamScore::One,
            2 => SpamScore::Two,
            _ => panic!(),
        };
        SpamEntry::WithoutSourceCommit((
            typed_score,
            NaiveDate::from_ymd_opt(year, month, day).unwrap(),
        ))
    }

    fn check_score_at_date(
        entries: &SpamEntries,
        year: i32,
        month: u32,
        day: u32,
        score: Option<u8>,
    ) {
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        let typed_score = score.map(|x| match x {
            0 => SpamScore::Zero,
            1 => SpamScore::One,
            2 => SpamScore::Two,
            _ => panic!(),
        });
        assert_eq!(entries.spam_score_at_date(date), typed_score);
    }

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

    #[test]
    pub fn basic_spam_entries() {
        let first = entry_without_source(2024, 1, 1, 0);
        let second = entry_without_source(2025, 1, 1, 1);
        let mut entries = SpamEntries::new(first);
        entries.add_spam_entry(second).unwrap();
        assert_eq!(entries.earliest_spam_entry(), first);
        assert_eq!(entries.last_spam_entry(), second);
        check_score_at_date(&entries, 2023, 12, 31, None);
        check_score_at_date(&entries, 2024, 1, 1, Some(0));
        check_score_at_date(&entries, 2024, 6, 1, Some(0));
        check_score_at_date(&entries, 2025, 1, 1, Some(1));
    }

    #[test]
    pub fn single_entry_spam_entries() {
        let first = entry_without_source(2023, 1, 1, 0);
        let entries = SpamEntries::new(first);
        check_score_at_date(&entries, 2022, 12, 31, None);
        check_score_at_date(&entries, 2023, 1, 1, Some(0));
        check_score_at_date(&entries, 2024, 12, 31, Some(0));
    }

    #[test]
    pub fn test_basic_serialization() {
        let label: SpamRecord = (SpamScore::One, NaiveDate::from_ymd_opt(2021, 5, 1).unwrap());
        let entries = SpamEntries::new(SpamEntry::WithoutSourceCommit(label));
        let json = json!(entries);
        let expected = r#"{"entries":[{"WithoutSourceCommit":["One","2021-05-01"]}],"version":1}"#;
        assert_eq!(json.to_string(), expected.to_string());
    }
}
