mod selection_rect;
use std::ops::{self, BitXor};

pub use selection_rect::SelectionRect;
mod node_search;
pub use node_search::NodeSearch;
pub mod drag_handler;
pub use drag_handler::{DragHandler, DragHandlerResult, Prepared, SelectablePath};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Select {
    #[default]
    Deselect,
    Select,
}
impl ops::Not for Select {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Self::Select => Self::Deselect,
            Self::Deselect => Self::Select,
        }
    }
}
impl Select {
    /// Commonly-used function to determine if a thing is selected (e.g. to apply conditional actions to selected things)
    pub fn is(self) -> bool {
        match self {
            Self::Select => true,
            Self::Deselect => false,
        }
    }
}

impl BitXor<bool> for Select {
    type Output = Self;
    fn bitxor(self, rhs: bool) -> Self::Output {
        if rhs { !self } else { self }
    }
}
