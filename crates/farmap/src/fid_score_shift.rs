use serde::Serialize;

#[derive(Serialize, Clone, Copy, PartialEq, Debug)]
pub struct FidScoreShift {
    source: ShiftSource,
    target: ShiftTarget,
    count: usize,
}

#[derive(Serialize, Clone, Copy, PartialEq, Debug)]
pub enum ShiftSource {
    Zero,
    One,
    Two,
    New,
}

#[derive(Serialize, Clone, Copy, PartialEq, Debug)]
pub enum ShiftTarget {
    Zero,
    One,
    Two,
    #[allow(dead_code)]
    Removed,
}

impl FidScoreShift {
    pub fn new(source: ShiftSource, target: ShiftTarget, count: usize) -> Self {
        Self {
            source,
            target,
            count,
        }
    }
}
