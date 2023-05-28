use std::env;
use std::ops::{Add, Div, Mul};
use std::time::Duration;

use bevy::math::Vec3Swizzles;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_prototype_debug_lines::{DebugLines, DebugLinesPlugin};
use bevy_spatial::kdtree::KDTree2;
use bevy_spatial::{AutomaticUpdate, SpatialAccess};
use rand::distributions::Uniform;
use rand::Rng;

const COHESION_STRENGTH: f32 = 0.2;

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
                        speed: 5.0,
                        rotation_speed: 2.0,
                        direction: get_random_direction(),
                        view_distance: 50.0,
                    },
                ));
            }
        }
    }
}

pub fn boid_cohesion_system(
    treeaccess: Res<NNTree>,
    mut boid_query: Query<(&mut Transform, &mut Boid, Entity), With<Boid>>,
    time: Res<Time>,
    mut lines: ResMut<DebugLines>,
) {
    for (mut transform, mut boid, entity) in boid_query.iter_mut() {
        let neighbors = treeaccess.within_distance(transform.translation.xy(), boid.view_distance);
        
        
        lines.line(
            transform.translation,
            boid.direction
                .mul(20.0)
                .extend(0.0)
                .add(transform.translation),
            0.01,
        );
        
        

        // if a new boid enters the view_distance then this point will snap to a new place.
        // we may therefore need to track a point for each boid and lerp towards the true average instead
        let avereage_point = calculate_average_point(neighbors, entity);

        if !avereage_point.eq(&Vec2::ZERO) {
            let vector_to_average_point = Vec2::new(
                avereage_point.x - transform.translation.x,
                avereage_point.y - transform.translation.y,
            );
            boid.direction = boid
                .direction
                .lerp(
                    vector_to_average_point,
                    COHESION_STRENGTH * time.delta_seconds(),
                )
                .normalize();
            

            
            lines.line(
                transform.translation,
                vector_to_average_point
                    .extend(0.0)
                    .add(transform.translation),
                0.1,
            );

            draw_x(&mut lines, avereage_point);
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
        let mut rotation_direction = 0.0;
        if keys.pressed(KeyCode::Left) {
            rotation_direction = 1.0
        } else if keys.pressed(KeyCode::Right) {
            rotation_direction = -1.0
        }

        boid.direction = rotate_vector(
            boid.direction,
            rotation_direction * boid.rotation_speed * time.delta_seconds(),
        )
        .normalize();
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

/*
fn calculate_average_point<T>(point_list: Vec<(Vec2, T)>, ignore: Vec2) -> Vec2 {
    let average_point = point_list.iter()
    .filter(|elem| elem.0 != ignore)
    .fold(Vec2::ZERO, |acc, x| acc + x.0);
    average_point.div(point_list.len() as f32)
}
*/

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
