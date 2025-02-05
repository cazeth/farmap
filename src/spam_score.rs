use crate::user::InvalidInputError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpamScore {
    Zero,
    One,
    Two,
}

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
