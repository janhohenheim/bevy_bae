# Bevy BAE (Behavior As Entities)

[![crates.io](https://img.shields.io/crates/v/bevy_bae)](https://crates.io/crates/bevy_bae)
[![docs.rs](https://docs.rs/bevy_bae/badge.svg)](https://docs.rs/bevy_bae)


BAE is an implementation of Hierarchical Task Networks (HTN) for Bevy, with a focus on composability, readability, and data-driven design.


What does Behavior as Entities mean? It means you define the AI's behavior as a regular old Bevy `Bundle`:

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_trunk_thumper_the_troll(mut commands: Commands) {
    commands.spawn((
        Name::new("Trunk Thumper, the mightly Troll"),
        Plan::new(),
        Select,
        tasks![
            // First priority: fight the enemy if we can see it
            (
                Name::new("Fight enemy"),
                Sequence,
                tasks![
                    (
                        conditions![Condition::eq("can_see_enemy", true),],
                        Operator::new(navigate_to_enemy),
                        effects![Effect::set("location", "enemy"),],
                    ),
                    Operator::new(do_trunk_slam),
                ],
            ),
            // Second priority: patrol bridges
            (
                Name::new("Patrol bridges"),
                Sequence,
                tasks![
                    Operator::new(choose_bridge_to_check),
                    (
                        Operator::new(navigate_to_bridge),
                        effects![Effect::set("location", "bridge"),],
                    ),
                    Operator::new(check_bridge),
                ],
            )
        ],
    ));
}

fn navigate_to_enemy(In(_input): In<OperatorInput>) -> OperatorStatus {
    // Your code goes here :)
    OperatorStatus::Success
}

fn do_trunk_slam(In(_input): In<OperatorInput>) -> OperatorStatus {
    OperatorStatus::Success
}

fn choose_bridge_to_check(In(_input): In<OperatorInput>) -> OperatorStatus {
    OperatorStatus::Success
}

fn navigate_to_bridge(In(_input): In<OperatorInput>) -> OperatorStatus {
    OperatorStatus::Success
}

fn check_bridge(In(_input): In<OperatorInput>) -> OperatorStatus {
    OperatorStatus::Success
}
```


## Concepts

BAE implements the [HTN](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter12_Exploring_HTN_Planners_through_Example.pdf) algorithm, aka Hierarchical Task Networks. This is a bit of a mix between behavior trees and planners like GOAP.

The heart of BAE is an entity holding a `Plan`:

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_npc(mut commands: Commands) {
    commands.spawn((
        Plan::new(),
    ));
}
```

Such entities will automatically have their `Plan` updated and executed for you. Of course, we need to tell BAE about what to actually *do* in the `Plan`! The simplest thing to do is to execute a system:

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_npc(mut commands: Commands) {
    commands.spawn((
        Plan::new(),
        Operator::new(greet),
    ));
}

fn greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    info!("Oh hai!");
    OperatorStatus::Success
}
```

This plan will contain a single system, `greet`, which will run and immediately return a success, which will advance the plan to the next step.
Since our plan only has one step, it will be completely finished at this point and replanned.
This means that this NPC will in effect spam its greeting every frame. A simple way to reduce spam is to keep the plan in the `greet` operator by telling BAE it's still in progress:

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_npc(mut commands: Commands) {
    commands.spawn((
        Plan::new(),
        Operator::new(greet),
    ));
}

fn greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    info!("Oh hai!");
    OperatorStatus::Ongoing
}
```

We can create more interesting behaviors by cobining our operators into *compound tasks*. Let's take a look at the `Sequence` task:

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_npc(mut commands: Commands) {
    commands.spawn((
        Plan::new(),
        Sequence,
        tasks![
            Operator::new(greet),
            Operator::new(idle),
        ]
    ));
}

fn greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    info!("Oh hai!");
    OperatorStatus::Success
}

fn idle(In(_input): In<OperatorInput>) -> OperatorStatus {
    OperatorStatus::Ongoing
}
```

A `Sequence` will use all valid subtasks and execute them in order. In this case, it will call `greet` once, then advance the plan, and finally stay forever in `idle`.

What does *valid* mean here? BAE uses [`bevy_mod_props`](https://github.com/NthTensor/bevy_mod_props), which attaches arbitrary key-value properties to entities.
BAE uses these properties to set up `Condition`s:

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_npc(mut commands: Commands) {
    commands.spawn((
        Plan::new(),
        conditions![Condition::eq("can_greet", true)],
        Operator::new(greet),
    ));
}

fn greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    info!("Oh hai!");
    OperatorStatus::Success
}
```

Spawning this plan will never run the `greet` system, as we never set the `can_greet` property on the entity. If a property is not set, it uses a default value, which is `false` in this case.
`Props` is a regular component on the entity, so there are multitudes of accessing and editing the properties. But `Commands` also has a handy method for this:

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_npc(mut commands: Commands) {
    commands
        .spawn((
            Plan::new(),
            conditions![Condition::eq("can_greet", true)],
            Operator::new(greet),
        ))
        .set_prop("can_greet", true);
}

fn greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    info!("Oh hai!");
    OperatorStatus::Success
}
```

Let's take this one step further and learn about the `Select` compound task. `Select` uses the first task that is valid:

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_npc(mut commands: Commands) {
    commands.spawn((
        Plan::new(),
        Select,
        tasks![
            (
                conditions![Condition::eq("can_greet", true)],
                Operator::new(greet)
            ),
            Operator::new(idle)
        ],
    ));
}

fn greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    info!("Oh hai!");
    OperatorStatus::Success
}

fn idle(In(_input): In<OperatorInput>) -> OperatorStatus {
    OperatorStatus::Ongoing
}
```

Here, `Select` will first try to plan the `greet` operator, but can't, since the `can_greet` property was never set. So, it falls back to the `idle` behavior.

The real spice in this comes from the fact that operators themselves can also change properties after they ran!

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_npc(mut commands: Commands) {
    commands.spawn((
        Plan::new(),
        Select,
        tasks![
            (
                conditions![Condition::eq("can_greet", true)],
                Operator::new(greet)
            ),
            (
                Operator::new(prepare_to_greet),
                effects![Effect::set("can_greet", true)],
            ),
        ],
    ));
}

fn greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    info!("Oh hai!");
    OperatorStatus::Success
}

fn prepare_to_greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    OperatorStatus::Success
}
```

Note that while we require conditions apply effects on operators here, the same can be done with compound tasks.

Let's consider what happens when running our app.
- First, `Select` will try to plan `greet`, but cannot, as `can_greet` is not set to `true`.
- Next, `Select` will plan `prepare_to_greet` as a fallback
- Once `prepare_to_greet` ran, `can_greet` is set to `true`.
- The plan ran out of operators, so it will replan
- `Select` will again try to plan `greet`, and this time it will be able to!
- `prepare_to_greet` will never be called again, as `can_greet` has a higher priority.

Conditions and effects are also "anticipated" correctly during planning. Let's go back to `Sequence` for a second:

```rust
use bevy::prelude::*;
use bevy_bae::prelude::*;

fn spawn_npc(mut commands: Commands) {
    commands.spawn((
        Plan::new(),
        Sequence,
        tasks![
            (
                Operator::new(prepare_to_greet),
                effects![Effect::set("can_greet", true)],
            ),
            (
                conditions![Condition::eq("can_greet", true)],
                Operator::new(greet)
            ),
        ],
    ));
}

fn greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    info!("Oh hai!");
    OperatorStatus::Success
}

fn prepare_to_greet(In(_input): In<OperatorInput>) -> OperatorStatus {
    OperatorStatus::Success
}
```

Here, `Sequence` will plan `prepare_to_greet` and knows that it will set `can_greet` after it runs,
so it also knows that `greet` will have its conditions fulfilled by the time it wants to run.
This means that `Sequence` can successfully include both `prepare_to_greet` *and* `greet` in the same plan!

And that's most there is to HTNs. The last important realization is that we can nest as many compound tasks as we want.
Scroll back up this document to the initial example we gave, which defines the behavior for Trunk Thumper the troll. Try to think through how our troll will behave.


## Terminology Notes

I used terminology that I felt was intuitive for a Bevy context. But if you're familiar with HTN, you may have scratched your head a bit at the explanation above.
The cheatsheet for how traditional HTN terminology maps to BAE is
- Domain: the entity holding the `Plan`, as well as its associated tree of relations.
- Primitive Task: The entity holding the `Operator` and its optional `Effects` and `Conditions`.
- Method: Combined into the entity holding the `CompoundTask`, associated `Tasks`, and optional `Effects` and `Conditions`.
- Compound Task: The `Tasks`.
  This was because it's the same as having a `Sequence` of an `Operator::noop` holding a condition, followed by a compound task.
- Operator: the system referenced by an `Operator`.

## Compatibility

| bevy        | bevy_bae |
|-------------|------------------------|
| 0.17        | 0.1                    |
