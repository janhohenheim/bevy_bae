//! Contains the [`UpdatePlan`] [`EntityEvent`].

use bevy_ecs::error::{DefaultErrorHandler, HandleError as _};
use bevy_ecs::system::command::run_system_cached_with;
use core::marker::PhantomData;

use crate::plan::PlannedOperator;
use crate::plan::mtr::Mtr;
use crate::prelude::*;
use crate::task::compound::{DecomposeInput, DecomposeResult, TypeErasedCompoundTask};

/// [`EntityEvent`] for updating a plan. Trigger this on an entity with a [`Plan`] to update its plan.
/// Updating it will only have an effect if the new plan found has a higher priority than the current one.
/// This ensures that ongoing [`Sequence`]s are not suddenly interrupted when updating the plan.
/// If you want to instead wipe the slate clean, insert [`Plan::new`] instead, or call [`Plan::clear`].
#[derive(EntityEvent)]
pub struct UpdatePlan {
    /// The entity holding the [`Plan`] to update.
    #[event_target]
    pub entity: Entity,
}

impl From<Entity> for UpdatePlan {
    fn from(entity: Entity) -> Self {
        Self { entity }
    }
}

impl UpdatePlan {
    /// Create a new [`UpdatePlan`] event for the given entity. Usually called with the [`EntityCommands::trigger`] API.
    pub fn new(entity: Entity) -> Self {
        Self::from(entity)
    }
}

/// Event triggered automatically when a plan is replaced.
#[derive(EntityEvent)]
pub struct ReplacePlan {
    /// The entity holding the [`Plan`] that was replaced.
    #[event_target]
    pub entity: Entity,
    /// The previous value of the [`Plan`]. To read the current value, query it in your observer.
    pub old: Plan,
    /// Here so users cannot accidentally create a [`ReplacePlan`] when they were
    /// actually looking for [`UpdatePlan`].
    _pd: PhantomData<()>,
}

pub(crate) fn update_plan(
    update: On<UpdatePlan>,
    mut commands: Commands,
    error_handler: Option<Res<DefaultErrorHandler>>,
) {
    let entity = update.entity;
    let error_handler = error_handler.map(|h| *h).unwrap_or_default();
    commands.queue(
        run_system_cached_with(update_plan_inner, UpdatePlan { entity })
            .handle_error_with(error_handler.0),
    );
}

fn update_plan_inner(
    update: In<UpdatePlan>,
    world: &mut World,
    mut conditions: Local<QueryState<(Entity, &Condition)>>,
    mut effects: Local<QueryState<Entity, With<Effect>>>,
    mut tasks: Local<
        QueryState<
            (Entity, Has<Operator>, Option<&TypeErasedCompoundTask>),
            Or<(With<Operator>, With<TypeErasedCompoundTask>)>,
        >,
    >,
) -> Result {
    let root = update.entity;

    let mut world_state = world.entity(update.entity).props().clone();
    let mut initial_conditions = Vec::new();
    if let Some(condition_relations) = world.get::<Conditions>(root) {
        for (entity, condition) in conditions.iter_many(world, condition_relations) {
            let is_fulfilled = condition.is_fullfilled(&mut world_state);
            if !is_fulfilled {
                world.entity_mut(root).insert(Plan::default());
                return Ok(());
            }
            initial_conditions.push(entity);
        }
    }

    let Ok((entity, has_operator, compound_task)) =
        tasks
            .get(world, root)
            .map(|(entity, has_operator, compound_task)| {
                (entity, has_operator, compound_task.cloned())
            })
    else {
        world.entity_mut(root).insert(Plan::default());
        return Err(BevyError::from("Called `update_plan` for an entity without any tasks. Ensure it has either an `Operator` or a `CompoundTask` like `Select` or `Sequence`".to_string()));
    };
    let mut plan = if has_operator {
        // well that was easy: this root has just a single operator
        Plan {
            operators_left: [PlannedOperator {
                entity,
                effects: vec![],
                conditions: initial_conditions,
            }]
            .into(),
            mtr: Mtr::default(),
            operators_total: Vec::new(),
        }
    } else if let Some(compound_task) = compound_task {
        let previous_mtr = if let Some(plan) = world.entity(root).get::<Plan>() {
            plan.mtr.clone()
        } else {
            Mtr::none()
        };
        let ctx = DecomposeInput {
            world_state,
            plan: Plan::default(),
            planner: root,
            compound_task: root,
            previous_mtr: previous_mtr.clone(),
            conditions: initial_conditions,
        };
        let result = world.run_system_with(compound_task.decompose, ctx)?;
        world.flush();

        match result {
            DecomposeResult::Success { plan, .. } => {
                if previous_mtr == plan.mtr
                    && world.entity(root).get::<Plan>().is_some_and(|prev_plan| {
                        prev_plan.operators_total.len() == plan.operators_left.len()
                            && prev_plan
                                .operators_total
                                .iter()
                                .zip(plan.operators_left.iter())
                                .all(|(a, b)| *a == b.entity)
                    })
                {
                    // We found the same plan we are already running. Just keep that one.
                    return Ok(());
                }
                plan
            }
            DecomposeResult::Failure => Plan::default(),
            DecomposeResult::Rejection => return Ok(()),
        }
    } else {
        unreachable!(
            "Bevy should guarantee that `AnyOf` contains at least one element that is `Some`"
        )
    };

    if !plan.is_empty()
        && let Some(effect_relations) = world.get::<Effects>(root)
    {
        for effect in effects.iter_many(world, effect_relations) {
            plan.back_mut().unwrap().effects.push(effect);
        }
    }

    let op_entities = plan
        .operators_left
        .iter()
        .map(|op| op.entity)
        .collect::<Vec<_>>();
    plan.operators_total = op_entities;

    let old_plan = world
        .entity(root)
        .get::<Plan>()
        .cloned()
        .unwrap_or_default();
    world.entity_mut(root).insert(plan);
    world.trigger(ReplacePlan {
        entity: root,
        old: old_plan,
        _pd: PhantomData,
    });
    Ok(())
}
