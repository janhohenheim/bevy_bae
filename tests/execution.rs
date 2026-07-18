//! Tests the plan execution

use bevy::{log::LogPlugin, prelude::*, time::TimeUpdateStrategy};
use bevy_bae::{plan::PlanDomain, prelude::*};
use bevy_ecs::entity_disabling::Disabled;
use std::sync::Mutex;

#[test]
fn runs_plan() {
    let mut app = App::test(op("a"));
    app.update();
    app.assert_last_opt("a");
}

#[test]
fn runs_plan_with_condition() {
    let mut app = App::test((Select, tasks![(op("a"), cond_is("use_a", true)), op("b")]));
    app.update();
    app.assert_last_opt("b");
}

#[test]
fn skips_plan_only_effect() {
    let mut app = App::test((
        Sequence,
        tasks![
            (op("a"), effects![Effect::set("use_b", true).plan_only()]),
            (op("b"), cond_is("use_b", true)),
        ],
    ));
    // plan
    app.update();
    app.assert_last_opt("a");

    // try to run b, but we didn't actually set use_b, so abort plan
    app.update();
    app.assert_last_opt(None);

    // replan same plan
    app.update();
    app.assert_last_opt("a");
}

#[test]
fn runs_plan_then_replans_with_new_effects() {
    let mut app = App::test((
        Select,
        tasks![
            (op("a"), cond_is("use_b", false), eff("use_b", true)),
            op("b")
        ],
    ));
    // plan
    app.update();
    app.assert_last_opt("a");
    // replan
    app.update();
    app.assert_last_opt("b");
    // replan to same plan
    app.update();
    app.assert_last_opt("b");
}

#[test]
fn replans_on_invalid_conditions() {
    let mut app = App::test((
        Select,
        tasks![
            (
                Sequence,
                tasks![op("a"), (op("b"), cond_is("disabled", false)),]
            ),
            op("c"),
        ],
    ));
    // plan
    app.update();
    app.assert_last_opt("a");
    app.behavior_entity().props_mut().set("disabled", true);
    // abort plan
    app.update();
    app.assert_last_opt(None);
    // replan
    app.update();
    app.assert_last_opt("c");
}

#[test]
fn ignores_disabled_behavior() {
    let mut app = App::test((
        Select,
        tasks![
            (op("a"), cond_is("use_b", false), eff("use_b", true)),
            op("b")
        ],
    ));
    // plan
    app.update();
    app.assert_last_opt("a");

    // disable behavior
    app.behavior_entity().insert(Disabled);
    app.update();
    app.assert_last_opt(None);

    // enable behavior and replan
    app.behavior_entity().remove::<Disabled>();
    app.update();
    app.assert_last_opt("b");
}

#[test]
fn replan_keeps_self() {
    let mut app = App::test((Sequence, tasks![op("a"), op("b")]));
    app.update();
    app.assert_last_opt("a");

    app.behavior_entity().trigger(UpdatePlan::new);

    app.update();
    app.assert_last_opt("b");
}

#[test]
fn replan_keeps_higher_priority() {
    let mut app = App::test((
        Select,
        tasks![
            (
                Sequence,
                tasks![
                    (op("a"), cond_is("disabled", false), eff("disabled", true)),
                    op("b")
                ]
            ),
            (Sequence, tasks![op("c"), op("d")])
        ],
    ));
    app.update();
    app.assert_last_opt("a");

    app.behavior_entity().trigger(UpdatePlan::new);

    // Even though our current sequence is no longer valid, we need to finish it before switching away due to a replan.
    app.update();
    app.assert_last_opt("b");

    app.update();
    app.assert_last_opt("c");

    app.update();
    app.assert_last_opt("d");
}

#[test]
fn replan_switches_to_higher_priority() {
    let mut app = App::test((
        Select,
        tasks![
            (
                Sequence,
                tasks![(op("a"), cond_is("enabled", true)), op("b")]
            ),
            (Sequence, tasks![(op("c"), eff("enabled", true)), op("d")])
        ],
    ));
    app.update();
    app.assert_last_opt("c");

    app.behavior_entity().trigger(UpdatePlan::new);

    app.update();
    app.assert_last_opt("a");

    app.update();
    app.assert_last_opt("b");

    app.update();
    app.assert_last_opt("a");
}

#[test]
fn does_not_replan_on_internal_prop_change() {
    let mut app = App::test((
        Select,
        tasks![
            (
                Sequence,
                tasks![(op("a"), cond_is("enabled", true)), op("b")]
            ),
            (Sequence, tasks![(op("c"), eff("enabled", true)), op("d")])
        ],
    ));
    app.update();
    app.assert_last_opt("c");

    app.update();
    app.assert_last_opt("d");

    app.update();
    app.assert_last_opt("a");

    app.update();
    app.assert_last_opt("b");

    app.update();
    app.assert_last_opt("a");
}

#[test]
fn does_not_replan_on_external_prop_change() {
    let mut app = App::test((
        Select,
        tasks![
            (
                Sequence,
                tasks![(op("a"), cond_is("enabled", true)), op("b")]
            ),
            (Sequence, tasks![op("c"), op("d")])
        ],
    ));
    app.update();
    app.assert_last_opt("c");

    app.behavior_entity().set_prop("enabled", true);

    app.update();
    app.assert_last_opt("d");

    app.update();
    app.assert_last_opt("a");

    app.update();
    app.assert_last_opt("b");

    app.update();
    app.assert_last_opt("a");
}

#[test]
fn higher_order_conditions_are_ignored_when_already_in_task() {
    let mut app = App::test((
        Select,
        tasks![
            (
                Sequence,
                cond_is("disabled", false),
                tasks![op("a"), op("b")]
            ),
            op("c")
        ],
    ));
    app.update();
    app.assert_last_opt("a");

    app.behavior_entity().set_prop("disabled", true);

    app.update();
    app.assert_last_opt("b");
}

#[test]
fn higher_order_conditions_are_respected_when_entering_task() {
    let mut app = App::test((
        Select,
        tasks![
            (
                Sequence,
                cond_is("disabled", false),
                tasks![op("a"), op("b")]
            ),
            op("c")
        ],
    ));

    app.behavior_entity().set_prop("disabled", true);
    app.update();
    app.assert_last_opt(None);
    app.update();
    app.assert_last_opt("c");
}

#[test]
fn compound_effects_are_applied() {
    let mut app = App::test((
        Select,
        tasks![
            (Sequence, tasks![op("a"), op("b")], eff("called", true),),
            op("c")
        ],
    ));
    app.update();
    app.assert_last_opt("a");

    assert!(!app.behavior_entity().get_prop::<bool>("called"));

    app.update();
    app.assert_last_opt("b");

    assert!(app.behavior_entity().get_prop::<bool>("called"));

    app.update();
    app.assert_last_opt("a");
}

#[test]
fn nested_compound_effects_are_applied() {
    let mut app = App::test((
        Select,
        eff("called_outer", true),
        tasks![
            (
                Sequence,
                eff("called_inner", true),
                tasks![op("a"), op("b")]
            ),
            op("c")
        ],
    ));
    app.update();
    app.assert_last_opt("a");

    assert!(!app.behavior_entity().get_prop::<bool>("called_outer"));
    assert!(!app.behavior_entity().get_prop::<bool>("called_inner"));

    app.update();
    app.assert_last_opt("b");

    assert!(app.behavior_entity().get_prop::<bool>("called_outer"));
    assert!(app.behavior_entity().get_prop::<bool>("called_inner"));

    app.update();
    app.assert_last_opt("a");
}

#[test]
fn compound_effects_are_not_applied_on_abort() {
    let mut app = App::test((
        Select,
        tasks![
            (
                Sequence,
                eff("called", true),
                tasks![op("a"), (op("b"), cond_is("disabled", false))]
            ),
            op("c")
        ],
    ));
    app.update();
    app.assert_last_opt("a");

    assert!(!app.behavior_entity().get_prop::<bool>("called"));

    app.behavior_entity().set_prop("disabled", true);
    app.update();
    app.assert_last_opt(None);
    assert!(!app.behavior_entity().get_prop::<bool>("called"));

    app.update();
    app.assert_last_opt("c");
    assert!(!app.behavior_entity().get_prop::<bool>("called"));
}

#[test]
fn logs_plan() {
    let mut app = App::test((
        Select,
        tasks![
            (
                Sequence,
                eff("called", true),
                tasks![op("a"), (op("b"), cond_is("disabled", false))]
            ),
            op("c")
        ],
    ));
    app.update();
    app.assert_last_opt("a");

    app.behavior_entity().trigger(LogPlan::new);

    app.update();
}

trait TestApp {
    fn test(behavior: impl Bundle) -> App;
    #[track_caller]
    fn assert_last_opt(&self, name: impl Into<Option<&'static str>>);
    fn behavior_entity(&mut self) -> EntityWorldMut<'_>;
}

impl TestApp for App {
    fn test(behavior: impl Bundle) -> App {
        let mut app = App::new();
        let behavior = Mutex::new(Some(behavior));
        app.add_plugins((
            MinimalPlugins,
            LogPlugin {
                filter: format!(
                    "bevy_log=off,bevy_bae=debug,{default}",
                    default = bevy::log::DEFAULT_FILTER
                ),
                ..default()
            },
            BaePlugin::default(),
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            Time::<Fixed>::default().timestep(),
        ))
        .init_resource::<LastOpt>()
        .add_systems(Startup, move |mut commands: Commands| {
            commands
                .spawn(behavior.lock().unwrap().take().unwrap())
                .insert_if_new(Name::new("root"))
                .insert_if_new(PlanDomain::default())
                .trigger(UpdatePlan::new);
        })
        .add_systems(PreUpdate, |mut last_opt: ResMut<LastOpt>| {
            last_opt.0 = None;
        });
        app.finish();
        app.update();
        app.assert_last_opt(None);
        app
    }

    #[track_caller]
    fn assert_last_opt(&self, expected: impl Into<Option<&'static str>>) {
        let expected: Option<&'static str> = expected.into();
        let expected: Option<String> = expected.map(Into::into);
        let actual = self.world().resource::<LastOpt>().0.clone();
        assert_eq!(expected, actual);
    }

    fn behavior_entity(&mut self) -> EntityWorldMut<'_> {
        let entity = self
            .world()
            .try_query_filtered::<Entity, (With<Plan>, Allow<Disabled>)>()
            .unwrap()
            .single(self.world())
            .unwrap();
        self.world_mut().entity_mut(entity)
    }
}
// The following functions are not reflective of real user code and are here to make the test suite more simple to set up.

#[derive(Resource, Default)]
struct LastOpt(Option<String>);

fn op(name: &str) -> impl Bundle {
    let name = name.to_string();
    (
        Name::new(name.clone()),
        Operator::new(
            move |_: In<OperatorInput>, mut last_opt: ResMut<LastOpt>| -> OperatorStatus {
                last_opt.0 = Some(name.clone());
                OperatorStatus::Success
            },
        ),
    )
}

fn cond_is(name: &str, val: impl Into<Value>) -> impl Bundle {
    conditions![Condition::eq(name, val)]
}

fn eff(name: &str, val: impl Into<Value>) -> impl Bundle {
    effects![Effect::set(name, val)]
}
