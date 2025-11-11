#![allow(unused)]
use chrono::Datelike;
use chrono::Days;
use chrono::Months;
use chrono::NaiveDate;

/// use this function to infallibly create dates throughout the library (for tests etc).
pub(crate) fn date(date: &str) -> NaiveDate {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("invalid internal date parse")
}

/// An iterator convenient for time series analysis
///
/// By default, it is an open-ended daily iterator.
/// In order to use this iterator, you will want modify the behaviour by calling appropriate methods and by finally using [`TimeIterator::build`]. After you have built the iterator, it is locked and you can no longer modify the behaviour. Only when the iterator is built can you actually iterate.
///
///
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeIterator<S, I: Default + IterInterval> {
    current: NaiveDate,
    end_date: Option<NaiveDate>,
    fused: bool,
    first: bool,
    time_specific: I,
    marker: std::marker::PhantomData<S>,
}

impl<S, I: Default + IterInterval> Default for TimeIterator<S, I> {
    fn default() -> Self {
        Self {
            first: true,
            end_date: None,
            fused: false,
            time_specific: I::default(),
            current: NaiveDate::default(),
            marker: std::marker::PhantomData,
        }
    }
}

impl TimeIterator<Unstarted, Daily> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<I: Default + IterInterval> TimeIterator<Unstarted, I> {
    pub fn with_start_date(mut self, start_date: NaiveDate) -> Self {
        self.current = start_date;
        self
    }

    pub fn with_end_date(mut self, end_date: NaiveDate) -> Self {
        self.end_date = Some(end_date);
        self
    }

    pub fn with_monthly_cadence(self) -> TimeIterator<Unstarted, Monthly> {
        let monthly = Monthly { day_in_month: 1 };
        TimeIterator::<Unstarted, Monthly> {
            current: self.current,
            end_date: self.end_date,
            fused: self.fused,
            first: self.first,
            time_specific: monthly,
            marker: std::marker::PhantomData,
        }
    }

    pub fn with_weekly_cadence(self) -> TimeIterator<Unstarted, Weekly> {
        TimeIterator::<Unstarted, Weekly> {
            current: self.current,
            end_date: self.end_date,
            fused: self.fused,
            first: self.first,
            time_specific: Weekly,
            marker: std::marker::PhantomData,
        }
    }

    pub fn build(self) -> TimeIterator<Ready, I> {
        TimeIterator::<Ready, I> {
            first: self.first,
            current: self.current,
            end_date: self.end_date,
            fused: self.fused,
            time_specific: self.time_specific,
            marker: std::marker::PhantomData,
        }
    }
}

impl<I: Default + IterInterval> TimeIterator<Ready, I> {
    pub fn handle_first_iteration(&mut self) -> Option<NaiveDate> {
        self.first = false;

        if self.end_date.is_some() && self.current == self.end_date.unwrap() {
            self.fused = true;
        }

        Some(self.current)
    }

    pub fn handle_non_first_iteration(&mut self) -> Option<NaiveDate> {
        match (
            self.end_date,
            self.time_specific.next_date_candidate(self.current)?,
        ) {
            (Some(end_date), next_date_candidate) if next_date_candidate < end_date => {
                self.current = next_date_candidate;
                Some(self.current)
            }
            (Some(end_date), next_date_candidate) if next_date_candidate == end_date => {
                self.current = next_date_candidate;
                self.fused = true;
                Some(self.current)
            }
            (Some(end_date), next_date_candidate) if next_date_candidate > end_date => {
                self.current = end_date;
                self.fused = true;
                Some(self.current)
            }
            (None, next_date_candidate) => {
                self.current = next_date_candidate;
                Some(self.current)
            }
            (_, _) => {
                unreachable!();
            }
        }
    }
}

impl TimeIterator<Unstarted, Monthly> {
    pub fn with_date_of_month(mut self, day_in_month: u8) -> Self {
        assert!(day_in_month < 28);
        self.time_specific.day_in_month = day_in_month;
        self
    }
}

#[allow(private_bounds)]
pub trait IterInterval: IterIntervalSeal {
    fn next_date_candidate(&self, previous_date: NaiveDate) -> Option<NaiveDate>;
}

trait IterIntervalSeal {}

pub struct Unstarted;
pub struct Ready;

#[derive(Default)]
pub struct Daily;

pub struct Monthly {
    day_in_month: u8,
}

#[derive(Default)]
pub struct Weekly;

impl IterIntervalSeal for Daily {}
impl IterIntervalSeal for Weekly {}
impl IterIntervalSeal for Monthly {}

impl IterInterval for Daily {
    fn next_date_candidate(&self, previous_date: NaiveDate) -> Option<NaiveDate> {
        previous_date.checked_add_days(Days::new(1))
    }
}

impl IterInterval for Weekly {
    fn next_date_candidate(&self, previous_date: NaiveDate) -> Option<NaiveDate> {
        previous_date.checked_add_days(Days::new(7))
    }
}

impl Default for Monthly {
    fn default() -> Self {
        Monthly { day_in_month: 1 }
    }
}

impl IterInterval for Monthly {
    fn next_date_candidate(&self, previous_date: NaiveDate) -> Option<NaiveDate> {
        previous_date
            .checked_add_months(Months::new(1))
            .and_then(|x| x.with_day(1))
    }
}

impl<I: IterInterval + Default> Iterator for TimeIterator<Ready, I> {
    type Item = NaiveDate;
    fn next(&mut self) -> Option<Self::Item> {
        if self.fused {
            return None;
        };

        if self.first {
            return self.handle_first_iteration();
        };

        self.handle_non_first_iteration()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_daily_no_end_date() {
        let count = TimeIterator::new().build().take(100).count();
        assert_eq!(count, 100);
    }

    #[test]
    fn test_monthly_no_end_date() {
        let count = TimeIterator::new()
            .with_start_date(date("2024-01-01"))
            .with_monthly_cadence()
            .build()
            .take(100)
            .count();
        assert_eq!(count, 100);
    }

    #[test]
    fn test_monthly_with_defaults() {
        let count = TimeIterator::new()
            .with_start_date(date("2024-01-15"))
            .with_end_date(date("2025-01-15"))
            .with_monthly_cadence()
            .build()
            .count();

        assert_eq!(count, 14);
    }
}
