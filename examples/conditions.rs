//! Shows how to use conditions and effects in BAE.
//! Once the player clicks the left mouse button, the NPC will start spamming greetings every frame.
//! When the player clicks the right mouse button, the NPC announces that they're done and stops spamming.

use bevy::prelude::*;
use bevy_bae::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin, BaePlugin::default()))
        .add_systems(Startup, setup)
        .add_observer(update_state)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Plan::new(),
        Select,
        tasks![
            (
                Operator::new(greet),
                conditions![Condition::eq("greet_mode", "on")]
            ),
            (
                Operator::new(say_stop),
                conditions![Condition::eq("greet_mode", "ending")],
                effects![Effect::set("greet_mode", false)]
            ),
            Operator::new(idle),
        ],
    ));
}

fn greet(_: In<OperatorInput>) -> OperatorStatus {
    info!("Oh hai!!! (Press right mouse button to stop the spam.)");
    OperatorStatus::Success
}

fn say_stop(_: In<OperatorInput>) -> OperatorStatus {
    info!("Ok ok I will stop now :<");
    OperatorStatus::Success
}

fn idle(_: In<OperatorInput>) -> OperatorStatus {
    // nothing to do
    OperatorStatus::Success
}

fn update_state(press: On<Pointer<Press>>, mut props: Single<&mut Props, With<Plan>>) {
    match press.button {
        PointerButton::Primary => props.set("greet_mode", "on"),
        PointerButton::Secondary if props["greet_mode"] == "on" => {
            props.set("greet_mode", "ending");
        }
        _ => (),
    }
}
