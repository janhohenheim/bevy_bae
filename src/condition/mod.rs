//! Contains the [`Condition`] component.

use alloc::sync::Arc;
use core::fmt::Debug;
use core::ops::RangeBounds;

use crate::prelude::*;

pub mod relationship;

/// A condition for an associated [`Operator`].
/// If the condition is unfulfilled, the compound task containing the [`Operator`] may be pruned.
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Condition {
    #[reflect(ignore, default = "Condition::true_pred")]
    predicate: Arc<dyn Fn(&mut Props) -> bool + Send + Sync + 'static>,
}

impl PartialEq for Condition {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.predicate, &other.predicate)
    }
}
impl Eq for Condition {}

impl Condition {
    /// Creates a new condition with the given predicate.
    pub fn new(predicate: impl Fn(&mut Props) -> bool + Send + Sync + 'static) -> Self {
        Self {
            predicate: Arc::new(predicate),
        }
    }

    /// Evaluates the condition with the given properties, returning whether it is fulfilled.
    /// It will insert props holding default values if they are queried, but are not yet present in [`Props`].
    pub fn is_fullfilled(&self, props: &mut Props) -> bool {
        (self.predicate)(props)
    }

    /// Shorthand for creating a condition for the concept of `props[name] == value`
    pub fn eq(name: impl Into<Estr>, value: impl Into<Value>) -> Self {
        Self::cmp(name, value, |a, b| a == b)
    }

    /// Shorthand for creating a condition for the concept of `props[name] != value`
    pub fn ne(name: impl Into<Estr>, value: impl Into<Value>) -> Self {
        Self::cmp(name, value, |a, b| a != b)
    }

    /// Shorthand for creating a condition for the concept of `props[name] > value`
    pub fn gt(name: impl Into<Estr>, value: impl Into<Value>) -> Self {
        Self::cmp(name, value, |a, b| a > b)
    }

    /// Shorthand for creating a condition for the concept of `props[name] >= value`
    pub fn ge(name: impl Into<Estr>, value: impl Into<Value>) -> Self {
        Self::cmp(name, value, |a, b| a >= b)
    }

    /// Shorthand for creating a condition for the concept of `props[name] < value`
    pub fn lt(name: impl Into<Estr>, value: impl Into<Value>) -> Self {
        Self::cmp(name, value, |a, b| a < b)
    }

    /// Shorthand for creating a condition for the concept of `props[name] <= value`
    pub fn le(name: impl Into<Estr>, value: impl Into<Value>) -> Self {
        Self::cmp(name, value, |a, b| a <= b)
    }

    /// Shorthand for creating a condition for the concept of `range.contains(props[name])`
    pub fn in_range(
        name: impl Into<Estr>,
        range: impl RangeBounds<f32> + Send + Sync + 'static,
    ) -> Self {
        let name = name.into();
        Self::new(move |props| range.contains(props.get_mut::<f32>(name)))
    }

    /// Shorthand for creating a condition that always evaluates to true
    pub fn always_true() -> Self {
        Self::new(|_| true)
    }

    /// Shorthand for creating a condition that always evaluates to false
    pub fn always_false() -> Self {
        Self::new(|_| false)
    }

    /// Shortcut for creating a condition that compares a property with a value.
    pub fn cmp(
        name: impl Into<Estr>,
        value: impl Into<Value>,
        predicate: impl Fn(Value, Value) -> bool + Send + Sync + 'static,
    ) -> Self {
        let name = name.into();
        let value = value.into();
        Self::new(move |p: &mut Props| predicate(*p.entry(name).or_default(), value))
    }

    fn true_pred() -> Arc<dyn Fn(&mut Props) -> bool + Send + Sync + 'static> {
        Arc::new(|_| true)
    }
}

impl Debug for Condition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Condition")
            .field("predicate", &"<callback>")
            .finish()
    }
}
