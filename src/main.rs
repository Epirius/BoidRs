use std::ops::{Add, Div, Mul, Neg};
use std::time::Duration;

use bevy::math::Vec3Swizzles;
use bevy::utils::HashMap;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin};
use bevy_spatial::kdtree::KDTree2;
use bevy_spatial::{AutomaticUpdate, SpatialAccess};
use rand::distributions::Uniform;
use rand::Rng;

const MANUAL_ROTATION_STRENGTH: f32 = 1.0;
const COHESION_STRENGTH: f32 = 0.2;
const ALINGMENT_STRENGTH: f32 = 0.2;
const SEPARATION_STRENGTH: f32 = 0.2;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugLinesPlugin::default())
        .add_plugin(
            AutomaticUpdate::<Boid>::new()
                .with_spatial_ds(bevy_spatial::SpatialStructure::KDTree2)
                .with_frequency(Duration::from_millis(1)),
        )
        .add_startup_system(spawn_camera)
        .add_system(spawn_boid)
        .add_system(move_boid_system)
        .add_system(rotate_boid_sprite_system)
        .add_system(rotate_boid_manual_system)
        .add_system(avoid_walls_system)
        .add_system(boid_cohesion_system)
        .add_system(boid_alignment_system)
        .add_system(boid_separation_system)
        .run();
}

pub fn spawn_camera(mut commands: Commands, window_query: Query<&Window, With<PrimaryWindow>>) {
    let window = window_query.get_single().unwrap();

    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, 0.0),
        ..default()
    });
}

type NNTree = KDTree2<Boid>;

#[derive(Component, Default)]
pub struct Boid {
    speed: f32,
    rotation_speed: f32,
    direction: Vec2,
    view_distance: f32,
    separation_distance: f32,
}

pub fn spawn_boid(
    mut commands: Commands,
    window_query: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    buttons: Res<Input<MouseButton>>,
) {
    if buttons.just_released(MouseButton::Left) {
        let window = window_query.get_single().unwrap();
        if buttons.just_released(MouseButton::Left) {
            if let Some(mouse_pos) = window.cursor_position() {
                let [x, y] = mouse_pos.to_array();
                commands.spawn((
                    SpriteBundle {
                        transform: Transform::from_xyz(x, y, 0.0),
                        texture: asset_server.load("sprites/boid01.png"),
                        ..default()
                    },
                    Boid {
                        speed: 20.0,
                        rotation_speed: 3.0,
                        direction: get_random_direction(),
                        view_distance: 50.0,
                        separation_distance: 20.0,
                    },
                ));
            }
        }
    }
}

pub fn boid_separation_system(
    treeaccess: Res<NNTree>,
    mut boid_query: Query<(&mut Transform, &mut Boid, Entity), With<Boid>>,
    time: Res<Time>,
){
    for (transform, mut boid, entity) in boid_query.iter_mut() {
        let neighbors = treeaccess.within_distance(transform.translation.xy(), boid.separation_distance);
        if neighbors.len() <= 1 {
            continue; // no neighbors.
        }
        let mut i = 0.0;
        let mut summed_vec_to_neighbors = Vec2::ZERO;
        for (pos, option) in neighbors {
            if option.is_some() && option.unwrap() == entity{
                continue; //skipping self
            }

            let vec_from_boid = Vec2::new(pos.x - transform.translation.x, pos.y - transform.translation.y);
            summed_vec_to_neighbors = summed_vec_to_neighbors.add(vec_from_boid);
            i += 1.0; 
        }
        let move_vec = summed_vec_to_neighbors.div(i).neg().normalize();
        let strength = boid.rotation_speed * time.delta_seconds() * SEPARATION_STRENGTH;
        rotate_boid_direction(&mut boid, move_vec, strength);
    }
}

// TODO alignment might also align speed if boids have different max speeds etc.
pub fn boid_alignment_system(
    treeaccess: Res<NNTree>,
    mut boid_query: Query<(&mut Transform, &mut Boid, Entity), With<Boid>>,
    time: Res<Time>,
) {
    let direction_map: HashMap<Entity, Vec2> = boid_query
        .iter()
        .map(|(_, boid, entity)| (entity, boid.direction))
        .collect();

    for (transform, mut boid, entity) in boid_query.iter_mut() {
        let neighbors = treeaccess.within_distance(transform.translation.xy(), boid.view_distance);

        let mut i: f32 = 0.0;
        let summed_direction = neighbors
            .iter()
            .filter_map(|(_, option)| *option)
            .filter(|e| e != &entity)
            .map(|e| {
                i += 1.0;
                direction_map.get(&e).unwrap()
            })
            .fold(Vec2::ZERO, |acc, vec| acc.add(*vec));

        if i == 0.0 {
            continue;
        };
        let average_direction = summed_direction.div(i);
        let strength = boid.rotation_speed * time.delta_seconds() * ALINGMENT_STRENGTH;
        rotate_boid_direction(&mut boid, average_direction, strength);
    }
}

pub fn boid_cohesion_system(
    treeaccess: Res<NNTree>,
    mut boid_query: Query<(&mut Transform, &mut Boid, Entity), With<Boid>>,
    time: Res<Time>,
    //mut lines: ResMut<DebugLines>,
) {
    for (mut transform, mut boid, entity) in boid_query.iter_mut() {
        let neighbors = treeaccess.within_distance(transform.translation.xy(), boid.view_distance);

        /*lines.line(
            transform.translation,
            boid.direction
                .mul(20.0)
                .extend(0.0)
                .add(transform.translation),
            0.01,
        );*/

        // if a new boid enters the view_distance then this point will snap to a new place.
        // we may therefore need to track a point for each boid and lerp towards the true average instead
        let avereage_point = calculate_average_point(neighbors, entity);

        if !avereage_point.eq(&Vec2::ZERO) {
            let vector_to_average_point = Vec2::new(
                avereage_point.x - transform.translation.x,
                avereage_point.y - transform.translation.y,
            );
            let strength = boid.rotation_speed * time.delta_seconds() * COHESION_STRENGTH;
            rotate_boid_direction(&mut boid, vector_to_average_point, strength);

            /*lines.line(
                transform.translation,
                vector_to_average_point
                    .extend(0.0)
                    .add(transform.translation),
                0.1,
            );*/

            //draw_x(&mut lines, avereage_point);
        }
    }
}

pub fn move_boid_system(
    mut boid_query: Query<(&mut Transform, &Boid), With<Boid>>,
    time: Res<Time>,
) {
    for (mut transform, boid) in boid_query.iter_mut() {
        transform.translation +=
            boid.direction.extend(0.0).normalize() * boid.speed * time.delta_seconds();
    }
}

pub fn rotate_boid_sprite_system(mut boid_query: Query<(&mut Transform, &Boid), With<Boid>>) {
    for (mut transform, boid) in boid_query.iter_mut() {
        transform.rotation = Quat::from_rotation_arc(Vec3::Y, boid.direction.extend(0.0));
    }
}

pub fn rotate_boid_manual_system(
    mut boid_query: Query<&mut Boid>,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
) {
    for mut boid in boid_query.iter_mut() {
        let rotation_vector = if keys.pressed(KeyCode::Left) {
            boid.direction.perp()
        } else if keys.pressed(KeyCode::Right) {
            boid.direction.perp().neg()
        } else {
            break;
        };
        let strength = boid.rotation_speed * time.delta_seconds() * MANUAL_ROTATION_STRENGTH;
        rotate_boid_direction(&mut boid, rotation_vector, strength);
    }
}

pub fn avoid_walls_system(
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut boid_query: Query<(&mut Transform, &Boid)>,
) {
    let window = window_query.get_single().unwrap();
    for (mut transform, _) in boid_query.iter_mut() {
        let [mut x, mut y] = transform.translation.xy().to_array();
        if x < 0.0 {
            x = window.width();
        } else if x > window.width() {
            x = 0.0;
        }
        if y < 0.0 {
            y = window.height();
        } else if y > window.height() {
            y = 0.0;
        }
        transform.translation = Vec3::new(x, y, 0.0);
    }
}

fn get_random_direction() -> Vec2 {
    let range = Uniform::new(0.0, 360.0);
    let mut rng = rand::thread_rng();
    let random_angle: f32 = rng.sample(range);
    let random_angle = random_angle.to_radians();
    Vec2::from_angle(random_angle)
}

fn rotate_vector(vector: Vec2, angle: f32) -> Vec2 {
    let cos_theta = angle.cos();
    let sin_theta = angle.sin();

    let x = vector.x * cos_theta - vector.y * sin_theta;
    let y = vector.x * sin_theta + vector.y * cos_theta;

    Vec2::new(x, y)
}

fn calculate_average_point(mut point_list: Vec<(Vec2, Option<Entity>)>, ignore: Entity) -> Vec2 {
    let average_point = point_list
        .iter_mut()
        .filter(|(vec, entity_option)| match entity_option {
            Some(entity) => entity != &ignore,
            None => true,
        })
        .fold(Vec2::ZERO, |acc, x| acc + x.0);

    // may want to remove the filter so that everyone in the same local group hase the same average point
    // ( remember to remove the -1 when deviding at the end of the function)

    if point_list.len() - 1 == 0 {
        return Vec2::ZERO;
    }
    average_point.div((point_list.len() - 1) as f32)
}

fn draw_x(mut lines: &mut ResMut<DebugLines>, point: Vec2) {
    let [x, y] = point.to_array();
    let left = Vec2::new(x - 3.0, y).extend(0.0);
    let right = Vec2::new(x + 3.0, y).extend(0.0);
    let top = Vec2::new(x, y + 3.0).extend(0.0);
    let bottom = Vec2::new(x, y - 3.0).extend(0.0);

    lines.line(left, right, 0.01);
    lines.line(top, bottom, 0.01);
}

fn rotate_boid_direction(boid: &mut Boid, target_vector: Vec2, strength: f32) {
    boid.direction = boid
        .direction
        .lerp(target_vector.normalize(), strength)
        .normalize();
}
