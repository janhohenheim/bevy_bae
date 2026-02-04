pub(crate) fn execute_plan(
    world: &mut World,
    mut plans: Local<QueryState<(NameOrEntity, &mut Plan)>>,
    mut conditions: Local<QueryState<(NameOrEntity, &Condition)>>,
    mut operators: Local<QueryState<(NameOrEntity, &Operator)>>,
    mut effects: Local<QueryState<(NameOrEntity, &Effect)>>,
    mut plans_scratch: Local<Vec<(Entity, Option<Name>, PlannedOperator)>>,
    mut condition_scratch: Local<Vec<(Entity, Option<Name>, Condition)>>,
    mut effects_scratch: Local<Vec<(Entity, Option<Name>, Effect)>>,
) {
    plans_scratch.extend(
        plans.iter(world).filter_map(|(name, plan)| {
            Some((name.entity, name.name.cloned(), plan.front()?.clone()))
        }),
    );
    for (plan_entity, plan_name, planned_operator) in plans_scratch.drain(..) {
        if !world.entity_mut(plan_entity).contains::<Props>() {
            world.entity_mut(plan_entity).insert(Props::default());
        }
        debug!(?plan_entity, ?plan_name, "checking conditions");
        let mut all_conditions_met = true;
        {
            condition_scratch.extend(
                conditions
                    .iter_many(world, planned_operator.conditions.iter())
                    .map(|(name, condition)| (name.entity, name.name.cloned(), condition.clone())),
            );
            let mut entity_mut = world.entity_mut(plan_entity);
            let mut props = entity_mut.get_mut::<Props>().unwrap();
            for (condition_entity, condition_name, condition) in condition_scratch.drain(..) {
                if condition.is_fullfilled(&mut props) {
                    debug!(
                        ?plan_entity,
                        ?plan_name,
                        ?condition_entity,
                        ?condition_name,
                        "satisfied condition"
                    );
                } else {
                    debug!(
                        ?plan_entity,
                        ?plan_name,
                        ?condition_entity,
                        ?condition_name,
                        "encountered unsatisfied condition, aborting plan"
                    );
                    all_conditions_met = false;
                    break;
                }
            }
        }
        let result: Result<OperatorStatus, _> = if all_conditions_met {
            let input = OperatorInput {
                entity: plan_entity,
                operator: planned_operator.entity,
            };
            if let Ok((op_name, operator)) = operators.get(world, planned_operator.entity) {
                debug!(
                    ?plan_entity,
                    ?plan_name,
                    operator_entity=?op_name.entity,
                    operator_name=?op_name.name,
                    "running operator"
                );
                let result = world.run_system_with(operator.system_id(), input);
                world.flush();
                result
            } else {
                debug!(
                    operator_entity=?planned_operator.entity,
                    "failed to find operator"
                );
                Ok(OperatorStatus::Failure)
            }
        } else {
            Ok(OperatorStatus::Failure)
        };

        let (force_replan, plan_entity_alive) = match result {
            Ok(OperatorStatus::Success) => {
                debug!(
                    ?plan_entity,
                    ?plan_name,
                    "operator completed successfully, moving to next step"
                );
                match world.get_entity_mut(plan_entity) {
                    Ok(mut entity_mut) => {
                        let step = entity_mut.get_mut::<Plan>().unwrap().pop_front().unwrap();

                        effects_scratch.extend(effects.iter_many(world, step.effects.iter()).map(
                            |(name, effect)| (name.entity, name.name.cloned(), effect.clone()),
                        ));
                        let mut entity = world.entity_mut(plan_entity);
                        let mut props = entity.get_mut::<Props>().unwrap();
                        for (effect_entity, effect_name, effect) in effects_scratch.drain(..) {
                            if effect.plan_only {
                                debug!(
                                    ?plan_entity,
                                    ?plan_name,
                                    ?effect_entity,
                                    ?effect_name,
                                    "skipping effect as it's plan_only"
                                );
                            } else {
                                debug!(
                                    ?plan_entity,
                                    ?plan_name,
                                    ?effect_entity,
                                    ?effect_name,
                                    "applying effect"
                                );
                                effect.apply(&mut props);
                            }
                        }

                        (false, true)
                    }
                    _ => (false, false),
                }
            }
            Ok(OperatorStatus::Ongoing) => {
                debug!(?plan_entity, ?plan_name, "operator ongoing");
                // Even if the current plan is empty, we still want to continue the execution of the last step!
                continue;
            }
            Ok(OperatorStatus::Failure) => {
                debug!(?plan_entity, ?plan_name, "operator failed, aborting plan");
                (true, world.get_entity(plan_entity).is_ok())
            }
            Err(err) => {
                debug!(
                    ?plan_entity,
                    ?plan_name,
                    ?err,
                    "operator system failed, aborting plan"
                );
                (true, world.get_entity(plan_entity).is_ok())
            }
        };
        if plan_entity_alive {
            let need_replan = force_replan
                || match world.get_entity(plan_entity) {
                    Ok(entity_ref) => entity_ref
                        .get::<Plan>()
                        .map_or(true, |plan| plan.is_empty()),
                    _ => false,
                };

            if need_replan {
                world.entity_mut(plan_entity).insert(Plan::default());
                debug!(?plan_entity, ?plan_name, "triggering replan");
            }
        }
    }
}
