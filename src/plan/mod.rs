//! Contains the [`Plan`] component and types for operating on it.

use alloc::collections::VecDeque;
use bevy_ecs::{entity_disabling::Disabled, lifecycle::HookContext, query::QueryEntityError, world::DeferredWorld};

use crate::{plan::mtr::Mtr, prelude::*};

pub(crate) mod execution;
pub mod mtr;
pub mod update;

/// Specifies the domain used by a planner entity.
#[derive(Component, Default)]
#[component(on_insert = Self::on_insert_hook)]
pub enum PlanDomain {
    /// The planner entity defines its own task hierarchy.
    #[default]
    This,
    /// The planner entity reuses a task hierarchy defined by another entity.
    Entity(Entity),
}

impl PlanDomain {
    fn on_insert_hook(mut world: DeferredWorld, context: HookContext) {
        let mut cmds = world.commands();
        cmds.entity(context.entity).insert_if_new(Plan::default());
    }
}

/// A full plan of operators to execute. If this is empty, either through manually clearing it, inserting it, when it runs out of operators, or fails to execute them,
/// the plan will be recomputed in the next fixed frame.
#[derive(Component, Clone, Default, PartialEq, Eq, Reflect, Debug, Deref, DerefMut)]
#[reflect(Component)]
#[require(Props, PlanDomain)]
pub struct Plan {
    /// The queue of planned [`Operator`]s to execute. This will get [`VecDeque::pop_front`]ed during plan execution.
    #[reflect(ignore)]
    #[deref]
    pub operators_left: VecDeque<PlannedOperator>,
    /// All [`Operator`]s that were in [`Plan::operators_left`] when the plan was created.
    pub operators_total: Vec<Entity>,
    /// The [`Mtr`] of the full plan when it was created.
    pub mtr: Mtr,
}

impl Plan {
    /// Creates a new empty plan. Inserting such a plan will immediately trigger a replan at the next fixed frame.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the current plan with a new empty plan. Doing so will immediately trigger a replan at the next fixed frame.
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

/// An entry in [`Plan::operators_left`], representing an operator that is either currently executing or waiting to execute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedOperator {
    /// The [`Entity`] of the [`Operator`].
    pub entity: Entity,
    /// The [`Effect`]s of the operator to be applied after it completes. Does not include effects that are [`Effect::plan_only`].
    /// The last operator of a compound task will also inherit effects from higher-up compound tasks.
    pub effects: Vec<Entity>,
    /// The [`Condition`]s that need to be fulfilled for the operator to be run.
    /// The first operator of a compound task will also inherit conditions from higher-up compound tasks.
    pub conditions: Vec<Entity>,
}

/// An [`EntityEvent`] for logging a given plan via [`info!`]
#[derive(EntityEvent, Debug)]
pub struct LogPlan {
    entity: Entity,
}

impl LogPlan {
    /// Creates a new [`LogPlan`] event for the given entity.
    pub fn new(entity: Entity) -> Self {
        LogPlan { entity }
    }
}

impl From<Entity> for LogPlan {
    fn from(entity: Entity) -> Self {
        LogPlan::new(entity)
    }
}

pub(crate) fn log_plan(
    log: On<LogPlan>,
    plans: Query<&Plan, Allow<Disabled>>,
    names: Query<NameOrEntity, Allow<Disabled>>,
) -> Result {
    let plan_entity = log.entity;
    let plan = plans.get(plan_entity)?;
    let name = |entity| -> Result<String, QueryEntityError> {
        names.get(entity).map(|n| n.entity_and_name())
    };
    let plan_name = name(plan_entity)?;
    let mut log = String::new();
    log.push_str(&format!("plan {plan_name}:\n"));
    log.push_str(&format!("- mtr: {}\n", plan.mtr));
    log.push_str(&format!(
        "- operators left ({}):\n",
        plan.operators_left.len()
    ));
    for operator in &plan.operators_left {
        let operator_name = name(operator.entity)?;
        log.push_str(&format!("  - {operator_name}:\n"));
        log.push_str(&format!("    - effects ({}):\n", operator.effects.len()));
        for effect in &operator.effects {
            let effect_name = name(*effect)?;
            log.push_str(&format!("      - {effect_name}\n"));
        }
        log.push_str(&format!(
            "    - conditions ({}):\n",
            operator.conditions.len()
        ));
        for condition in &operator.conditions {
            let condition_name = name(*condition)?;
            log.push_str(&format!("      - {condition_name}\n"));
        }
    }
    log.push_str(&format!(
        "- total operators ({})\n",
        plan.operators_total.len()
    ));
    for operator in &plan.operators_total {
        let operator_name = name(*operator)?;
        log.push_str(&format!("  - {operator_name}\n"));
    }
    info!("{}", log.trim());
    Ok(())
}
