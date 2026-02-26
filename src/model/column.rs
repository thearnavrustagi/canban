use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ColumnKind {
    Ready,
    Doing,
    Done,
    Archived,
}

impl ColumnKind {
    pub const ALL: [ColumnKind; 4] = [
        ColumnKind::Ready,
        ColumnKind::Doing,
        ColumnKind::Done,
        ColumnKind::Archived,
    ];

    pub fn next(self) -> Option<ColumnKind> {
        match self {
            Self::Ready => Some(Self::Doing),
            Self::Doing => Some(Self::Done),
            Self::Done => Some(Self::Archived),
            Self::Archived => None,
        }
    }

    pub fn prev(self) -> Option<ColumnKind> {
        match self {
            Self::Ready => None,
            Self::Doing => Some(Self::Ready),
            Self::Done => Some(Self::Doing),
            Self::Archived => Some(Self::Done),
        }
    }
}

impl fmt::Display for ColumnKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready => write!(f, "Ready"),
            Self::Doing => write!(f, "Doing"),
            Self::Done => write!(f, "Done"),
            Self::Archived => write!(f, "Archived"),
        }
    }
}
