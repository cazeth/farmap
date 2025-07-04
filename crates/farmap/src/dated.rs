//! create a dated version of any type.
use std::ops::{Deref, DerefMut};

use chrono::NaiveDate;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub struct Dated<T> {
    #[serde(flatten)]
    inner: T,
    date: NaiveDate,
}

impl<T> Dated<T> {
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> From<(T, NaiveDate)> for Dated<T> {
    fn from(value: (T, NaiveDate)) -> Self {
        Self {
            inner: value.0,
            date: value.1,
        }
    }
}

impl<T: Copy> Copy for Dated<T> {}

impl<T: Clone> Clone for Dated<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            date: self.date,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Dated<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dated")
            .field("inner", &self.inner)
            .field("date", &self.date)
            .finish()
    }
}

impl<T: PartialEq> PartialEq for Dated<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self.date == other.date
    }
}

impl<T: Default> Dated<T> {
    pub fn default_with_date(date: NaiveDate) -> Self {
        Self {
            inner: T::default(),
            date,
        }
    }

    pub fn map_into<S>(self) -> Dated<S>
    where
        S: From<T>,
    {
        Dated::<S> {
            date: self.date(),
            inner: self.inner.into(),
        }
    }

    pub fn try_map_into<S>(self) -> Result<Dated<S>, S::Error>
    where
        S: TryFrom<T>,
    {
        let new_date = self.date();
        let new_inner: S = self.inner.try_into()?;
        Ok(Dated::<S> {
            date: new_date,
            inner: new_inner,
        })
    }
    pub fn from<S>(date: NaiveDate, arg: S) -> Self
    where
        T: From<S>,
    {
        let inner = T::from(arg);
        Self { inner, date }
    }
}

impl<T> Deref for Dated<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Dated<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
