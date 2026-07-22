//! Contains [`Operator`] and associated types

use core::fmt::Debug;

use bevy_ecs::system::SystemId;
use bevy_ecs::{lifecycle::HookContext, world::DeferredWorld};

use crate::prelude::*;
use crate::task::validation::BaeTaskPresent;

/// The exact type of [`SystemId`] valid for [`Operator`]s.
pub type OperatorId = SystemId<In<OperatorInput>, OperatorStatus>;

/// The smallest unit of a plan, representing a single step. Contains a system that gets called for you during the execution of the plan.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[component(on_insert = Self::on_insert_hook, on_discard = Self::on_discard_hook)]
#[require(BaeTaskPresent)]
pub struct Operator {
    #[reflect(ignore)]
    register_system: Option<Box<dyn FnOnce(&mut Commands) -> OperatorId + Send + Sync>>,
    #[reflect(ignore)]
    system_id: Option<OperatorId>,
}

impl Clone for Operator {
    fn clone(&self) -> Self {
        Self {
            register_system: None,
            system_id: self.system_id,
        }
    }
}

impl PartialEq for Operator {
    fn eq(&self, other: &Self) -> bool {
        self.system_id == other.system_id
    }
}

impl Eq for Operator {}

impl Debug for Operator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Operator")
            .field("system_id", &self.system_id)
            .finish()
    }
}

impl Operator {
    /// Creates a new operator using the provided system. The system must take [`OperatorInput`] as input and return an [`OperatorStatus`].
    pub fn new<S, M>(system: S) -> Self
    where
        S: IntoSystem<In<OperatorInput>, OperatorStatus, M>,
        S::System: Send + Sync + 'static,
    {
        let system = IntoSystem::into_system(system);
        Self {
            system_id: None,
            register_system: Some(Box::new(move |commands| commands.register_system(system))),
        }
    }

    /// Shorthand for creating an operator that does nothing.
    pub fn noop() -> Self {
        Self::new(|_: In<OperatorInput>| OperatorStatus::Success)
    }

    /// Returns the [`SystemId`] of the registered operator one-shot system.
    pub fn system_id(&self) -> OperatorId {
        self.system_id.unwrap()
    }

    fn on_insert_hook(mut world: DeferredWorld, context: HookContext) {
        let Some(register_system) = world
            .get_mut::<Self>(context.entity)
            .and_then(|mut task_system| task_system.register_system.take())
        else {
            return;
        };
        let system_id = register_system(&mut world.commands());
        world.get_mut::<Self>(context.entity).unwrap().system_id = Some(system_id);
    }

    fn on_discard_hook(mut world: DeferredWorld, context: HookContext) {
        let Some(system_id) = world
            .get::<Self>(context.entity)
            .and_then(|tt| tt.system_id)
        else {
            return;
        };
        world.commands().unregister_system(system_id);
    }
}

/// Inputs for an operator.
pub struct OperatorInput {
    /// The entity up the hierarchy that holds the [`Plan`]. This is usually your entity of interest.
    pub entity: Entity,
    /// The entity that represents the operator itself. Useful if you want to associate custom extra data with an operator.
    pub operator: Entity,
}
