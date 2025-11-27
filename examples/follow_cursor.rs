//! A small setup showing how to code the behavior for a 2D triangle that chases the cursor
//! and starts rotation when close to it.

use bevy::{color::palettes::tailwind, picking::pointer::PointerInteraction, prelude::*};
use bevy_bae::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin, BaePlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
            update_close_to_cursor.before(BaeSystems::ExecutePlan),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);
    commands.spawn((
        Name::new("Background"),
        Mesh2d(meshes.add(Rectangle::new(10000.0, 10000.0))),
        MeshMaterial2d(materials.add(Color::from(tailwind::GRAY_200))),
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));
    commands.spawn((
        Name::new("NPC"),
        Mesh2d(meshes.add(Triangle2d::new(
            Vec2::Y * 20.0,
            Vec2::new(-20.0, -20.0),
            Vec2::new(20.0, -20.0),
        ))),
        MeshMaterial2d(materials.add(Color::from(tailwind::ROSE_500))),
        Plan::new(),
        Select,
        tasks![
            (
                conditions![Condition::eq("close_to_cursor", true)],
                Operator::new(rotate),
            ),
            Operator::new(follow_cursor),
        ],
    ));
}

fn rotate(In(input): In<OperatorInput>, mut transforms: Query<&mut Transform>) -> OperatorStatus {
    let mut npc_transform = transforms.get_mut(input.entity).unwrap();
    npc_transform.rotate_z(0.1);
    OperatorStatus::Ongoing
}

fn follow_cursor(
    In(input): In<OperatorInput>,
    pointers: Query<&PointerInteraction>,
    mut transforms: Query<&mut Transform>,
) -> OperatorStatus {
    let mut npc_transform = transforms.get_mut(input.entity).unwrap();
    for point in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position)
    {
        let dir = (point - npc_transform.translation).normalize();
        npc_transform.align(Vec3::NEG_Z, Vec3::NEG_Z, Vec3::Y, dir);
        npc_transform.translation += dir * 2.5;
    }
    OperatorStatus::Ongoing
}

fn update_close_to_cursor(
    pointers: Query<&PointerInteraction>,
    npc: Single<(Entity, &Transform, &mut Props), With<Plan>>,
    mut commands: Commands,
) {
    let (npc, npc_transform, mut props) = npc.into_inner();
    for point in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position)
    {
        let is_close = point.distance_squared(npc_transform.translation) < 10.0 * 10.0;
        if props["close_to_cursor"] != is_close {
            props.set("close_to_cursor", is_close);
            commands.entity(npc).trigger(UpdatePlan::new);
        }
    }
}
