use bevy::{prelude::*, window::PrimaryWindow};
use rand::distributions::Uniform;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(spawn_camera)
        .add_system(spawn_boid)
        .add_system(move_boid_system)
        .add_system(rotate_boid_sprite_system)
        .add_system(rotate_boid_system)
        .run();
}

pub fn spawn_camera(mut commands: Commands, window_query: Query<&Window, With<PrimaryWindow>>) {
    let window = window_query.get_single().unwrap();

    commands.spawn(Camera2dBundle {
        transform: Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, 0.0),
        ..default()
    });
}

#[derive(Component)]
pub struct Boid {
    speed: f32,
    rotation_speed: f32,
    direction: Vec2,
}

impl Boid {
    fn get_vec3(&self) -> Vec3 {
        Vec3::new(self.direction.x, self.direction.y, 0.0)
    }
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
                        speed: 50.0,
                        rotation_speed: 2.0,
                        direction: get_random_direction(),
                    },
                ));
            }
        }
    }
}

pub fn move_boid_system(
    mut boid_query: Query<(&mut Transform, &Boid), With<Boid>>,
    time: Res<Time>,
) {
    for (mut transform, boid) in boid_query.iter_mut() {
        let move_direction = boid.get_vec3();
        transform.translation += move_direction.normalize() * boid.speed * time.delta_seconds();
    }
}

pub fn rotate_boid_sprite_system(mut boid_query: Query<(&mut Transform, &Boid), With<Boid>>) {
    for (mut transform, boid) in boid_query.iter_mut() {
        transform.rotation = Quat::from_rotation_arc(Vec3::Y, boid.get_vec3());
    }
}

pub fn rotate_boid_system(
    mut boid_query: Query<&mut Boid>,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
) {
    for mut boid in boid_query.iter_mut() {
        let mut rotation_direction = 0.0;     
        if keys.pressed(KeyCode::Left) {
            rotation_direction = 1.0
        } else if keys.pressed(KeyCode::Right){
            rotation_direction = -1.0
        }
        
        
        boid.direction = rotate_vector(boid.direction, rotation_direction * boid.rotation_speed * time.delta_seconds());
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