//! Types for dealing with [`Operator`] effects. See [`Effect`] for more information.

use crate::prelude::*;
use alloc::sync::Arc;
use core::fmt::Debug;

pub mod relationship;

/// An effect on the properties of the entity holding [`Plan`]. These effects are taken into account during planning,
/// and applied automatically when the associated step of the plan succeeds.
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct Effect {
    #[reflect(ignore, default = "Effect::noop")]
    effect: Arc<dyn Fn(&mut Props) + Send + Sync + 'static>,
    /// Whether the effect should be taken into account only during planning, but not applied for you.
    /// Default is `false`, i.e. all effects are applied when the associated step of the plan succeeds.
    pub plan_only: bool,
}

impl PartialEq for Effect {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.effect, &other.effect) && self.plan_only == other.plan_only
    }
}

impl Eq for Effect {}

impl Effect {
    /// Creates a new effect from the given function.
    pub fn new(fun: impl Fn(&mut Props) + Send + Sync + 'static) -> Self {
        Self {
            effect: Arc::new(fun),
            plan_only: false,
        }
    }

    /// Ensures that the effect is taken into account for planning, but not applied for you.
    /// This is useful for effects that come from the outside world, such as "did the monster find the player?".
    /// This is off by default, i.e. all effects are applied when the associated step of the plan succeeds.
    pub fn plan_only(mut self) -> Self {
        self.plan_only = true;
        self
    }

    /// Applies the effect to the given properties.
    pub fn apply(&self, props: &mut Props) {
        (self.effect)(props);
    }

    /// Shortcut for creating an effect that sets a property.
    pub fn set(name: impl Into<Estr>, value: impl Into<Value>) -> Self {
        let name = name.into();
        let value = value.into();
        Self::new(move |props| props.set(name, value))
    }

    /// Shortcut for creating an effect that toggles a boolean property.
    /// If the property didn't exist before, it will be initialized to `true`.
    pub fn toggle(name: impl Into<Estr>) -> Self {
        let name = name.into();

        Self::new(move |props| {
            let val = props.get_mut::<bool>(name);
            *val = !*val;
        })
    }

    /// Shortcut for creating an effect that increments a numeric property.
    /// If the property didn't exist before, it will be initialized to `value`.
    pub fn inc<T: Into<Value>>(name: impl Into<Estr>, value: impl Into<Value>) -> Self {
        Self::mutate(name, value, |a, b| *a += b)
    }

    /// Shortcut for creating an effect that increments a numeric property.
    /// If the property didn't exist before, it will be initialized to `-value`.
    pub fn dec<T: Into<Value> + Default>(
        name: impl Into<Estr>,
        value: impl Into<Value> + Default,
    ) -> Self {
        Self::mutate(name, value, |a, b| *a -= b)
    }

    /// Shortcut for creating an effect that multiplies a numeric property.
    /// If the property didn't exist before, it will be initialized to `0`.
    pub fn mul(name: impl Into<Estr>, value: impl Into<Value> + Default) -> Self {
        Self::mutate(name, value, |a, b| *a *= b)
    }

    /// Shortcut for creating an effect that divides a numeric property.
    /// If the property didn't exist before, it will be initialized to `0`.
    pub fn div(name: impl Into<Estr>, value: impl Into<Value> + Default) -> Self {
        Self::mutate(name, value, |a, b| *a /= b)
    }

    /// Shortcut for creating an effect that modifies a property based on a value.
    pub fn mutate(
        name: impl Into<Estr>,
        value: impl Into<Value>,
        mutate: impl Fn(&mut Value, Value) + Send + Sync + 'static,
    ) -> Self {
        let name = name.into();
        let value = value.into();
        Self::new(move |props| {
            let prop = props.entry(name).or_default();
            mutate(prop, value);
        })
    }

    /// Shortcut for creating an effect that does nothing. This is equivalent to just not spawning an effect at all.
    pub fn noop() -> Arc<dyn Fn(&mut Props) + Send + Sync + 'static> {
        Arc::new(|_| {})
    }
}

impl Debug for Effect {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Effect")
            .field("effect", &"<callback>")
            .finish()
    }
}
