//! Contains the [`Mtr`] struct.

use core::cmp::Ordering;
use core::fmt::{self, Display};

use crate::prelude::*;

/// Method Traversal Record
#[derive(Clone, Default, Reflect, Debug, Deref, DerefMut)]
pub struct Mtr(pub Vec<u16>);

impl Display for Mtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let string = self
            .0
            .iter()
            .map(|&x| x.to_string())
            .collect::<Vec<_>>()
            .join(".");
        write!(f, "{string}")
    }
}

impl Mtr {
    /// Adds a new method order to the MTR. Lower values mean higher priority.
    pub fn with(mut self, method_order: u16) -> Self {
        self.push(method_order);
        self
    }

    /// Creates an empty MTR that will be considered lower priority than any other MTR.
    pub fn none() -> Self {
        Mtr(vec![])
    }
}

impl PartialEq for Mtr {
    fn eq(&self, other: &Self) -> bool {
        self.0.iter().zip(other.0.iter()).all(|(a, b)| a == b)
    }
}

impl Eq for Mtr {}

impl PartialOrd for Mtr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_empty() {
            return Some(Ordering::Greater);
        }
        if other.is_empty() {
            return Some(Ordering::Less);
        }
        for (a, b) in self.0.iter().zip(other.0.iter()) {
            match a.cmp(b) {
                Ordering::Equal => continue,
                ord => return Some(ord),
            }
        }
        Some(Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anything_is_better_than_nothing() {
        let mtr1 = Mtr(vec![1, 2, 3]);
        let mtr2 = Mtr(vec![]);

        assert!(mtr1 < mtr2);
    }

    #[test]
    fn self_is_le_and_ge() {
        let mtr1 = Mtr(vec![1, 2, 3]);
        let mtr2 = Mtr(vec![1, 2, 3]);

        assert!(mtr1 <= mtr2);
        assert!(mtr1 >= mtr2);
    }

    #[test]
    fn last_of_same_length_works() {
        let mtr1 = Mtr(vec![1, 2, 2]);
        let mtr2 = Mtr(vec![1, 2, 3]);

        assert!(mtr1 < mtr2);
    }

    #[test]
    fn mid_of_same_length_works() {
        let mtr1 = Mtr(vec![1, 1, 3]);
        let mtr2 = Mtr(vec![1, 2, 3]);

        assert!(mtr1 < mtr2);
    }

    #[test]
    fn start_of_same_length_works() {
        let mtr1 = Mtr(vec![0, 2, 3]);
        let mtr2 = Mtr(vec![1, 2, 3]);

        assert!(mtr1 < mtr2);
    }

    #[test]
    fn better_but_shorter_works() {
        let mtr1 = Mtr(vec![1, 2, 3]);
        let mtr2 = Mtr(vec![1, 2, 4, 5]);

        assert!(mtr1 < mtr2);
    }

    #[test]
    fn better_but_longer_works() {
        let mtr1 = Mtr(vec![1, 2, 3, 5]);
        let mtr2 = Mtr(vec![1, 2, 4]);

        assert!(mtr1 < mtr2);
    }

    #[test]
    fn same_but_longer_is_equal() {
        let mtr1 = Mtr(vec![1, 2, 3]);
        let mtr2 = Mtr(vec![1, 2, 3, 5]);

        assert!(mtr1 == mtr2);
    }
}
