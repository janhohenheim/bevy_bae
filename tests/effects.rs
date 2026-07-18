//! Tests the application of effects

use bevy::{log::LogPlugin, prelude::*, time::TimeUpdateStrategy};
use bevy_bae::{plan::{Plan, PlanDomain}, prelude::*};
use std::sync::Mutex;

#[test]
fn behavior_operator_has_no_effects() {
    assert_effects(op("a"), vec![vec![]]);
}

#[test]
fn behavior_operator_applies_effects() {
    assert_effects((op("a"), eff("foo")), vec![vec![("foo", true.into())]]);
}

#[test]
fn empty_plan_has_no_effects() {
    assert_effects((cond(false), op("a"), eff("foo")), vec![]);
}

#[test]
fn empty_plan_from_sequence_has_no_effects() {
    assert_effects((Sequence, tasks![]), vec![]);
}

#[test]
fn sequence_effects() {
    assert_effects(
        (
            Sequence,
            tasks![
                (op("a"), eff("foo")),
                (op("b"), eff("bar")),
                (op("c"), eff("baz")),
            ],
        ),
        vec![
            vec![("foo", true.into())],
            vec![("bar", true.into())],
            vec![("baz", true.into())],
        ],
    );
}

#[test]
fn sequence_effects_overwriting() {
    assert_effects(
        (
            Sequence,
            tasks![
                (op("a"), eff("foo")),
                (op("b"), eff_set("foo", "1")),
                (op("c"), eff_set("foo", 2.0)),
            ],
        ),
        vec![
            vec![("foo", true.into())],
            vec![("foo", "1".into())],
            vec![("foo", 2.0.into())],
        ],
    );
}

#[test]
fn sequence_effects_stacked() {
    assert_effects(
        (
            Sequence,
            tasks![
                (
                    op("a"),
                    effects![Effect::set("a", true), Effect::set("b", true)]
                ),
                (
                    op("b"),
                    effects![Effect::set("c", true), Effect::set("d", true)]
                ),
                (
                    op("c"),
                    effects![Effect::set("e", true), Effect::set("f", true)]
                ),
            ],
        ),
        vec![
            vec![("a", true.into()), ("b", true.into())],
            vec![("c", true.into()), ("d", true.into())],
            vec![("e", true.into()), ("f", true.into())],
        ],
    );
}

#[test]
fn select_effects() {
    assert_effects(
        (
            Select,
            tasks![
                (op("a"), cond(false), eff("foo")),
                (op("b"), eff("bar")),
                (op("c"), eff("baz")),
            ],
        ),
        vec![vec![("bar", true.into())]],
    );
}

#[test]
fn select_effects_undoes_invalid_effects() {
    assert_effects(
        (
            Select,
            tasks![
                (
                    Sequence,
                    tasks![(op("a"), eff("foo")), (op("b"), cond(false), eff("bar")),]
                ),
                (op("c"), cond(false), eff("baz")),
                (op("d"), eff("quux")),
            ],
        ),
        vec![vec![("quux", true.into())]],
    );
}

#[track_caller]
fn assert_effects(behavior: impl Bundle, props: Vec<Vec<(&'static str, Value)>>) {
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

    let actual_props = actual_plan
        .operators_left
        .into_iter()
        .map(|planned_op| {
            planned_op
                .effects
                .into_iter()
                .map(|effect| {
                    let effect = app.world().entity(effect).get::<Effect>().unwrap();
                    let mut props = Props::new();
                    effect.apply(&mut props);
                    props
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let expected_props = props
        .into_iter()
        .map(|props| {
            props
                .into_iter()
                .map(|(name, value)| {
                    let mut props = Props::new();
                    props.set(name, value);
                    props
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        expected_props.len(),
        actual_props.len(),
        "expected: {:?}, actual: {:?}",
        expected_props,
        actual_props
    );

    for (i, (expected, actual)) in expected_props
        .into_iter()
        .zip(actual_props.into_iter())
        .enumerate()
    {
        assert_eq!(
            expected.len(),
            actual.len(),
            "{i}: expected: {:?}, actual: {:?}",
            expected,
            actual
        );
        for (j, (expected_props, actual_props)) in
            expected.into_iter().zip(actual.into_iter()).enumerate()
        {
            assert_eq!(
                expected_props.iter().len(),
                actual_props.iter().len(),
                "{i}.{j}: expected: {:?}, actual: {:?}",
                expected_props,
                actual_props
            );
            for ((expected_key, expected_value), (actual_key, actual_value)) in
                expected_props.into_iter().zip(actual_props.into_iter())
            {
                assert_eq!(expected_key, actual_key);
                assert_eq!(expected_value, actual_value);
            }
        }
    }
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

fn eff(name: &str) -> impl Bundle {
    effects![Effect::set(name, true)]
}

fn eff_set(name: &str, value: impl Into<Value>) -> impl Bundle {
    effects![Effect::set(name, value)]
}
