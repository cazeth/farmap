use crate::SpamScore;
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize, Clone, Copy, PartialEq, Debug, Hash, Eq)]
pub struct FidScoreShift {
    source: ShiftSource,
    target: ShiftTarget,
    count: usize,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShiftSource {
    Zero,
    One,
    Two,
    New,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShiftTarget {
    Zero,
    One,
    Two,
    #[allow(dead_code)]
    Removed,
}

impl TryFrom<usize> for FidScoreShift {
    type Error = InvalidNumberError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FidScoreShift::new(ShiftSource::Zero, ShiftTarget::Zero, 0)),
            1 => Ok(FidScoreShift::new(ShiftSource::Zero, ShiftTarget::One, 0)),
            2 => Ok(FidScoreShift::new(ShiftSource::Zero, ShiftTarget::Two, 0)),
            3 => Ok(FidScoreShift::new(ShiftSource::One, ShiftTarget::Zero, 0)),
            4 => Ok(FidScoreShift::new(ShiftSource::One, ShiftTarget::One, 0)),
            5 => Ok(FidScoreShift::new(ShiftSource::One, ShiftTarget::Two, 0)),
            6 => Ok(FidScoreShift::new(ShiftSource::Two, ShiftTarget::Zero, 0)),
            7 => Ok(FidScoreShift::new(ShiftSource::Two, ShiftTarget::One, 0)),
            8 => Ok(FidScoreShift::new(ShiftSource::Two, ShiftTarget::Two, 0)),
            9 => Ok(FidScoreShift::new(ShiftSource::New, ShiftTarget::Zero, 0)),
            10 => Ok(FidScoreShift::new(ShiftSource::New, ShiftTarget::One, 0)),
            11 => Ok(FidScoreShift::new(ShiftSource::New, ShiftTarget::Two, 0)),
            _ => Err(InvalidNumberError),
        }
    }
}

impl TryFrom<FidScoreShift> for usize {
    type Error = InvalidNumberError;
    fn try_from(value: FidScoreShift) -> Result<usize, Self::Error> {
        match (value.source(), value.target()) {
            (ShiftSource::Zero, ShiftTarget::Zero) => Ok(0),
            (ShiftSource::Zero, ShiftTarget::One) => Ok(1),
            (ShiftSource::Zero, ShiftTarget::Two) => Ok(2),
            (ShiftSource::One, ShiftTarget::Zero) => Ok(3),
            (ShiftSource::One, ShiftTarget::One) => Ok(4),
            (ShiftSource::One, ShiftTarget::Two) => Ok(5),
            (ShiftSource::Two, ShiftTarget::Zero) => Ok(6),
            (ShiftSource::Two, ShiftTarget::One) => Ok(7),
            (ShiftSource::Two, ShiftTarget::Two) => Ok(8),
            (ShiftSource::New, ShiftTarget::Zero) => Ok(9),
            (ShiftSource::New, ShiftTarget::One) => Ok(10),
            (ShiftSource::New, ShiftTarget::Two) => Ok(11),
            _ => Err(InvalidNumberError),
        }
    }
}

impl TryFrom<u8> for ShiftSource {
    type Error = InvalidNumberError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::New),
            _ => Err(InvalidNumberError),
        }
    }
}

impl TryFrom<u8> for ShiftTarget {
    type Error = InvalidNumberError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Removed),
            _ => Err(InvalidNumberError),
        }
    }
}

impl From<SpamScore> for ShiftSource {
    fn from(value: SpamScore) -> Self {
        match value {
            SpamScore::Zero => ShiftSource::Zero,
            SpamScore::One => ShiftSource::One,
            SpamScore::Two => ShiftSource::Two,
        }
    }
}

impl From<SpamScore> for ShiftTarget {
    fn from(value: SpamScore) -> Self {
        match value {
            SpamScore::Zero => ShiftTarget::Zero,
            SpamScore::One => ShiftTarget::One,
            SpamScore::Two => ShiftTarget::Two,
        }
    }
}

#[derive(Error, Debug)]
#[error("number out of bounds, must be between 0 and 16")]
pub struct InvalidNumberError;

impl FidScoreShift {
    pub fn new(source: ShiftSource, target: ShiftTarget, count: usize) -> Self {
        Self {
            source,
            target,
            count,
        }
    }

    pub fn source(&self) -> ShiftSource {
        self.source
    }

    pub fn add(&mut self) {
        self.count += 1;
    }

    pub fn target(&self) -> ShiftTarget {
        self.target
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }
}
