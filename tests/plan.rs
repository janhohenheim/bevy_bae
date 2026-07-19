//! Tests the plan generation

use bevy::{log::LogPlugin, prelude::*, time::TimeUpdateStrategy};
use bevy_bae::{plan::{Plan, PlanDomain}, prelude::*};
use std::sync::Mutex;

#[test]
fn behavior_operator() {
    assert_plan(op("a"), vec!["a"]);
}

#[test]
#[should_panic]
fn behavior_empty() {
    assert_plan((), vec![]);
}

#[test]
fn sequence_single() {
    assert_plan((Sequence, tasks![op("a")]), vec!["a"]);
}

#[test]
fn sequence_single_fail() {
    assert_plan((Sequence, tasks![(cond(false), op("a"))]), vec![]);
}

#[test]
fn sequence_multi() {
    assert_plan((Sequence, tasks![op("a"), op("b")]), vec!["a", "b"]);
}

#[test]
fn sequence_multi_fail() {
    assert_plan((Sequence, tasks![op("a"), (cond(false), op("b"))]), vec![]);
}

#[test]
fn sequence_empty() {
    assert_plan((Sequence, tasks![]), vec![]);
}

#[test]
fn sequence_nested_1() {
    assert_plan(
        (
            Sequence,
            tasks![(Sequence, tasks![op("a"), op("b")]), op("c")],
        ),
        vec!["a", "b", "c"],
    );
}

#[test]
fn sequence_nested_2() {
    assert_plan(
        (
            Sequence,
            tasks![op("a"), (Sequence, tasks![op("b"), op("c")]),],
        ),
        vec!["a", "b", "c"],
    );
}

#[test]
fn sequence_nested_3() {
    assert_plan(
        (
            Sequence,
            tasks![
                (Sequence, tasks![op("a"), op("b")]),
                (Sequence, tasks![op("c"), op("d")]),
            ],
        ),
        vec!["a", "b", "c", "d"],
    );
}

#[test]
fn sequence_nested_fail() {
    assert_plan(
        (
            Sequence,
            tasks![
                (Sequence, tasks![op("a"), (cond(false), op("b"))]),
                (Sequence, tasks![op("c"), op("d")]),
            ],
        ),
        vec![],
    );
}

#[test]
fn sequence_disabled_by_effects() {
    assert_plan(
        (
            Sequence,
            tasks![
                (op("a"), eff("disabled", true)),
                (op("b"), cond_is("disabled", false)),
            ],
        ),
        vec![],
    );
}

#[test]
fn sequence_enabled_by_effects() {
    assert_plan(
        (
            Sequence,
            tasks![
                (op("a"), eff("enabled", true)),
                (op("b"), cond_is("enabled", true)),
            ],
        ),
        vec!["a", "b"],
    );
}

#[test]
fn select_single() {
    assert_plan((Select, tasks![op("a")]), vec!["a"]);
}

#[test]
fn select_first() {
    assert_plan((Select, tasks![op("a"), op("b")]), vec!["a"]);
}

#[test]
fn select_empty() {
    assert_plan((Select, tasks![]), vec![]);
}

#[test]
fn select_first_conditional() {
    assert_plan((Select, tasks![(cond(true), op("a")), op("b")]), vec!["a"]);
}

#[test]
fn select_second() {
    assert_plan((Select, tasks![(cond(false), op("a")), op("b")]), vec!["b"]);
}

#[test]
fn select_second_conditional() {
    assert_plan(
        (
            Select,
            tasks![(cond(false), op("a")), (cond(true), op("b")),],
        ),
        vec!["b"],
    );
}

#[test]
fn select_nested() {
    assert_plan(
        (
            Select,
            tasks![
                (cond(false), op("a")),
                (Select, tasks![(cond(false), op("b")), op("c")]),
            ],
        ),
        vec!["c"],
    );
}

#[test]
fn sequence_and_select() {
    assert_plan(
        (
            Select,
            tasks![
                (Sequence, tasks![op("a"), (cond(false), op("b"))]),
                (
                    Select,
                    tasks![(cond(false), op("c")), (Sequence, tasks![op("d"), op("e")])]
                ),
            ],
        ),
        vec!["d", "e"],
    );
}

#[test]
fn effect_not_disabled_by_invalid_branch() {
    assert_plan(
        (
            Select,
            tasks![
                (
                    Sequence,
                    tasks![(op("a"), eff("disabled", true)), (cond(false), op("b"))]
                ),
                (
                    Select,
                    tasks![
                        (cond(false), op("c")),
                        (
                            Sequence,
                            tasks![(cond_is("disabled", false), op("d")), op("e")]
                        )
                    ]
                ),
            ],
        ),
        vec!["d", "e"],
    );
}

#[track_caller]
fn assert_plan(behavior: impl Bundle, plan: Vec<&'static str>) {
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
    .add_systems(Startup, move |mut commands: Commands| {
        commands
            .spawn(behavior.lock().unwrap().take().unwrap())
            .insert_if_new(Name::new("root"))
            .insert_if_new(PlanDomain::default())
            .trigger(UpdatePlan::new);
    });
    app.finish();
    app.update();
    let actual_plan = app
        .world()
        .try_query::<&Plan>()
        .unwrap()
        .single(app.world())
        .unwrap()
        .clone();

    let mut operators = app
        .world()
        .try_query_filtered::<(Entity, &Name), With<Operator>>()
        .unwrap();
    let actual_plan_names = actual_plan
        .operators_left
        .into_iter()
        .map(|planned_op| {
            operators
                .iter(app.world())
                .find_map(|(op, name)| (op == planned_op.entity).then(|| name.to_string()))
                .unwrap()
        })
        .collect::<Vec<_>>();

    let plan_names = plan
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    assert_eq!(plan_names, actual_plan_names);
}

// The following functions are not reflective of real user code and are here to make the test suite more simple to set up.

fn op(name: &str) -> impl Bundle {
    (Name::new(name.to_string()), Operator::noop())
}

fn cond(val: bool) -> impl Bundle {
    conditions![if val {
        Condition::always_true()
    } else {
        Condition::always_false()
    }]
}

fn cond_is(name: &str, val: impl Into<Value>) -> impl Bundle {
    conditions![Condition::eq(name, val)]
}

fn eff(name: &str, val: impl Into<Value>) -> impl Bundle {
    effects![Effect::set(name, val)]
}
