//! A simplified implementation of the classic game "Breakout".
//!
//! Demonstrates Bevy's stepping capabilities if compiled with the `bevy_debug_stepping` feature.

use bevy::{
    math::bounding::{Aabb2d, BoundingCircle, BoundingVolume, IntersectsVolume},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};

mod stepping;

// These constants are defined in `Transform` units.
// Using the default 2D camera they correspond 1:1 with screen pixels.
const PADDLE_SIZE: Vec2 = Vec2::new(120.0, 20.0);
const GAP_BETWEEN_PADDLE_AND_FLOOR: f32 = 60.0;
const PADDLE_SPEED: f32 = 500.0;
// How close can the paddle get to the wall
const PADDLE_PADDING: f32 = 10.0;

// We set the z-value of the ball to 1 so it renders on top in the case of overlapping sprites.
const BALL_STARTING_POSITION: Vec3 = Vec3::new(0.0, -50.0, 1.0);
const BALL_DIAMETER: f32 = 30.;
const BALL_SPEED: f32 = 400.0;
const INITIAL_BALL_DIRECTION: Vec2 = Vec2::new(0.5, -0.5);

const WALL_THICKNESS: f32 = 10.0;
// x coordinates
const LEFT_WALL: f32 = -450.;
const RIGHT_WALL: f32 = 450.;
// y coordinates
const BOTTOM_WALL: f32 = -300.;
const TOP_WALL: f32 = 300.;

const BRICK_SIZE: Vec2 = Vec2::new(100., 30.);
// These values are exact
const GAP_BETWEEN_PADDLE_AND_BRICKS: f32 = 270.0;
const GAP_BETWEEN_BRICKS: f32 = 5.0;
// These values are lower bounds, as the number of bricks is computed
const GAP_BETWEEN_BRICKS_AND_CEILING: f32 = 20.0;
const GAP_BETWEEN_BRICKS_AND_SIDES: f32 = 20.0;

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const BACKGROUND_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const PADDLE_COLOR: Color = Color::srgb(0.3, 0.3, 0.7);
const BALL_COLOR: Color = Color::srgb(1.0, 0.5, 0.5);
const BRICK_COLOR: Color = Color::srgb(0.5, 0.5, 1.0);
const WALL_COLOR: Color = Color::srgb(0.8, 0.8, 0.8);
const TEXT_COLOR: Color = Color::srgb(0.5, 0.5, 1.0);
const SCORE_COLOR: Color = Color::srgb(1.0, 0.5, 0.5);

struct Level {
    brick_layout: Vec<Vec<Option<Brick>>>,
}

// Define the levels using fixed-size arrays

fn create_level_1() -> Level {
    Level {
        brick_layout: vec![
            vec![Some(Brick), None, Some(Brick)],
            vec![Some(Brick), Some(Brick), Some(Brick)],
            vec![Some(Brick), Some(Brick), Some(Brick)],
            vec![Some(Brick), Some(Brick), Some(Brick)],
        ],
    }
}

fn create_level_2() -> Level {
    Level {
        brick_layout: vec![
            vec![Some(Brick), Some(Brick), Some(Brick)],
            vec![Some(Brick), None, Some(Brick)],
            vec![Some(Brick), Some(Brick), Some(Brick)],
            vec![Some(Brick), Some(Brick), Some(Brick)],
        ],
    }
}


#[derive(Resource)]
struct GameState {
    levels: Vec<Level>,
    current_level: usize,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            levels: vec![create_level_1(), create_level_2()],
            current_level: 0,
        }
    }
}


fn next_level(game_state: &mut GameState) {
    if game_state.current_level + 1 < game_state.levels.len() {
        game_state.current_level += 1;
    } else {
        println!("You have completed all levels!");
    }
}



fn main() {
    App::new()
        // Insert resources first
        .insert_resource(GameState::default())
        .insert_resource(Score(0))
        .insert_resource(ClearColor(BACKGROUND_COLOR))

        // Add plugins after inserting resources
        .add_plugins(DefaultPlugins)
        .add_plugins(
            stepping::SteppingPlugin::default()
                .add_schedule(Update)
                .add_schedule(FixedUpdate)
                .at(Val::Percent(35.0), Val::Percent(50.0)),
        )

        // Register events
        .add_event::<CollisionEvent>()

        // Add systems
        .add_systems(Startup, setup)
        // Add our gameplay simulation systems to the fixed timestep schedule
        // which runs at 64 Hz by default
        .add_systems(
            FixedUpdate,
            (
                apply_velocity,
                move_paddle,
                check_for_collisions,
                play_collision_sound,
            )
                // `chain`ing systems together runs them in order
                .chain(),
        )
        .add_systems(Update, update_scoreboard)
        .run();
}

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Ball;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Collider;

#[derive(Event, Default)]
struct CollisionEvent;

#[derive(Component)]
struct Brick;

#[derive(Resource, Deref)]
struct CollisionSound(Handle<AudioSource>);

// This bundle is a collection of the components that define a "wall" in our game
#[derive(Bundle)]
struct WallBundle {
    // You can nest bundles inside of other bundles like this
    // Allowing you to compose their functionality
    sprite_bundle: SpriteBundle,
    collider: Collider,
}

/// Which side of the arena is this wall located on?
enum WallLocation {
    Left,
    Right,
    Bottom,
    Top,
}

impl WallLocation {
    /// Location of the *center* of the wall, used in `transform.translation()`
    fn position(&self) -> Vec2 {
        match self {
            WallLocation::Left => Vec2::new(LEFT_WALL, 0.),
            WallLocation::Right => Vec2::new(RIGHT_WALL, 0.),
            WallLocation::Bottom => Vec2::new(0., BOTTOM_WALL),
            WallLocation::Top => Vec2::new(0., TOP_WALL),
        }
    }

    /// (x, y) dimensions of the wall, used in `transform.scale()`
    fn size(&self) -> Vec2 {
        let arena_height = TOP_WALL - BOTTOM_WALL;
        let arena_width = RIGHT_WALL - LEFT_WALL;
        // Make sure we haven't messed up our constants
        assert!(arena_height > 0.0);
        assert!(arena_width > 0.0);

        match self {
            WallLocation::Left | WallLocation::Right => {
                Vec2::new(WALL_THICKNESS, arena_height + WALL_THICKNESS)
            }
            WallLocation::Bottom | WallLocation::Top => {
                Vec2::new(arena_width + WALL_THICKNESS, WALL_THICKNESS)
            }
        }
    }
}

impl WallBundle {
    // This "builder method" allows us to reuse logic across our wall entities,
    // making our code easier to read and less prone to bugs when we change the logic
    fn new(location: WallLocation) -> WallBundle {
        WallBundle {
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    // We need to convert our Vec2 into a Vec3, by giving it a z-coordinate
                    // This is used to determine the order of our sprites
                    translation: location.position().extend(0.0),
                    // The z-scale of 2D objects must always be 1.0,
                    // or their ordering will be affected in surprising ways.
                    // See https://github.com/bevyengine/bevy/issues/4149
                    scale: location.size().extend(1.0),
                    ..default()
                },
                sprite: Sprite {
                    color: WALL_COLOR,
                    ..default()
                },
                ..default()
            },
            collider: Collider,
        }
    }
}

// This resource tracks the game's score
#[derive(Resource, Deref, DerefMut)]
struct Score(usize);

#[derive(Component)]
struct ScoreboardUi;

fn load_level(level: &Level, commands: &mut Commands) {
    // Determine the number of bricks per row and total rows
    let bricks_per_row = level.brick_layout[0].len();
    let total_rows = level.brick_layout.len();

    // Calculate total width of a row: (brick width * number of bricks) + (gap * (number of bricks - 1))
    let total_width = bricks_per_row as f32 * BRICK_SIZE.x + (bricks_per_row as f32 - 1.) * GAP_BETWEEN_BRICKS;

    // Starting x position to center bricks horizontally
    let start_x = -total_width / 2. + BRICK_SIZE.x / 2.;

    // Starting y position near the top wall
    let start_y = TOP_WALL - GAP_BETWEEN_BRICKS_AND_CEILING - BRICK_SIZE.y / 2.;

    for (row_idx, row) in level.brick_layout.iter().enumerate() {
        for (brick_idx, brick) in row.iter().enumerate() {
            if brick.is_some() {
                // Calculate brick position
                let x = start_x + brick_idx as f32 * (BRICK_SIZE.x + GAP_BETWEEN_BRICKS);
                let y = start_y - row_idx as f32 * (BRICK_SIZE.y + GAP_BETWEEN_BRICKS);

                // Spawn brick with Collider
                commands.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: BRICK_COLOR,
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: Vec3::new(x, y, 0.0),
                        scale: Vec3::new(BRICK_SIZE.x, BRICK_SIZE.y, 1.0),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                  .insert(Brick)
                  .insert(Collider); // Ensure Collider is added
            }
        }
    }
}




// Add the game's entities to our world
use bevy::prelude::*;
use bevy::sprite::Material2d;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut game_state: ResMut<GameState>,
    mut materials: ResMut<Assets<ColorMaterial>>, // Added parameter
) {
    // Initialize the game state
    game_state.current_level = 0;

    // Camera
    commands.spawn(Camera2dBundle::default());

    // Sound Resource for Ball Collision
    let ball_collision_sound = asset_server.load("sounds/breakout_collision.ogg");
    commands.insert_resource(CollisionSound(ball_collision_sound));

    // Paddle
    let paddle_y = BOTTOM_WALL + GAP_BETWEEN_PADDLE_AND_FLOOR;
    commands.spawn((
        SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, paddle_y, 0.0),
                scale: PADDLE_SIZE.extend(1.0),
                ..default()
            },
            sprite: Sprite {
                color: PADDLE_COLOR,
                ..default()
            },
            ..default()
        },
        Paddle,
        Collider,
    ));

    // Ball with ColorMaterial
    commands.spawn((
        MaterialMesh2dBundle::<ColorMaterial> { // Specified ColorMaterial
            mesh: meshes.add(Mesh::from(Circle::new(BALL_DIAMETER / 2.0))).into(),
            material: materials.add(ColorMaterial::from(BALL_COLOR)), // Assigned material
            transform: Transform::from_translation(BALL_STARTING_POSITION)
              .with_scale(Vec3::new(1.0, 1.0, 1.0)),
            ..default()
        },
        Ball,
        Velocity(INITIAL_BALL_DIRECTION.normalize() * BALL_SPEED),
    ));

    // Scoreboard
    commands.spawn((
        ScoreboardUi,
        TextBundle::from_sections([
            TextSection::new(
                "Score: ",
                TextStyle {
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: TEXT_COLOR,
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                },
            ),
            TextSection::from_style(TextStyle {
                font_size: SCOREBOARD_FONT_SIZE,
                color: SCORE_COLOR,
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            }),
        ])
          .with_style(Style {
              position_type: PositionType::Absolute,
              margin: UiRect { // Changed from `position` to `margin`
                  top: SCOREBOARD_TEXT_PADDING,
                  left: SCOREBOARD_TEXT_PADDING,
                  ..default()
              },
              ..default()
          }),
    ));

    // Walls
    commands.spawn(WallBundle::new(WallLocation::Left));
    commands.spawn(WallBundle::new(WallLocation::Right));
    commands.spawn(WallBundle::new(WallLocation::Bottom));
    commands.spawn(WallBundle::new(WallLocation::Top));

    // Bricks for the initial level
    load_level(&game_state.levels[game_state.current_level], &mut commands);
}




fn move_paddle(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Paddle>>,
    time: Res<Time>,
) {
    let mut paddle_transform = query.single_mut();
    let mut direction = 0.0;

    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        direction -= 1.0;
    }

    if keyboard_input.pressed(KeyCode::ArrowRight) {
        direction += 1.0;
    }

    // Calculate the new horizontal paddle position based on player input
    let new_paddle_position =
        paddle_transform.translation.x + direction * PADDLE_SPEED * time.delta_seconds();

    // Update the paddle position,
    // making sure it doesn't cause the paddle to leave the arena
    let left_bound = LEFT_WALL + WALL_THICKNESS / 2.0 + PADDLE_SIZE.x / 2.0 + PADDLE_PADDING;
    let right_bound = RIGHT_WALL - WALL_THICKNESS / 2.0 - PADDLE_SIZE.x / 2.0 - PADDLE_PADDING;

    paddle_transform.translation.x = new_paddle_position.clamp(left_bound, right_bound);
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time.delta_seconds();
        transform.translation.y += velocity.y * time.delta_seconds();
    }
}

fn update_scoreboard(score: Res<Score>, mut query: Query<&mut Text, With<ScoreboardUi>>) {
    let mut text = query.single_mut();
    text.sections[1].value = score.to_string();
}

fn check_for_collisions(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut ball_query: Query<(&mut Velocity, &Transform), With<Ball>>,
    collider_query: Query<(Entity, &Transform, Option<&Brick>), With<Collider>>,
    mut collision_events: EventWriter<CollisionEvent>,
    mut game_state: ResMut<GameState>,
    brick_query: Query<Entity, With<Brick>>,
) {
    let (mut ball_velocity, ball_transform) = ball_query.single_mut();

    for (collider_entity, collider_transform, maybe_brick) in &collider_query {
        let collision = ball_collision(
            BoundingCircle::new(ball_transform.translation.truncate(), BALL_DIAMETER / 2.),
            Aabb2d::new(
                collider_transform.translation.truncate(),
                collider_transform.scale.truncate() / 2.,
            ),
        );

        if let Some(collision) = collision {
            println!("Collision detected with Entity: {:?}", collider_entity);
            // Sends a collision event so that other systems can react to the collision
            collision_events.send_default();

            // Bricks should be despawned and increment the scoreboard on collision
            if maybe_brick.is_some() {
                println!("Brick hit! Despawning brick: {:?}", collider_entity);
                commands.entity(collider_entity).despawn();
                **score += 1;
            }

            // Reflect the ball's velocity when it collides
            let mut reflect_x = false;
            let mut reflect_y = false;

            // Reflect only if the velocity is in the opposite direction of the collision
            // This prevents the ball from getting stuck inside the bar
            match collision {
                Collision::Left => reflect_x = ball_velocity.x > 0.0,
                Collision::Right => reflect_x = ball_velocity.x < 0.0,
                Collision::Top => reflect_y = ball_velocity.y < 0.0,
                Collision::Bottom => reflect_y = ball_velocity.y > 0.0,
            }

            // Reflect velocity on the x-axis if we hit something on the x-axis
            if reflect_x {
                println!("Reflecting ball velocity on the X-axis");
                ball_velocity.x = -ball_velocity.x;
            }

            // Reflect velocity on the y-axis if we hit something on the y-axis
            if reflect_y {
                println!("Reflecting ball velocity on the Y-axis");
                ball_velocity.y = -ball_velocity.y;
            }
        }
    }

    if all_bricks_are_cleared(brick_query) {
        if game_state.current_level + 1 < game_state.levels.len() {
            println!("All bricks cleared! Proceeding to next level.");
            next_level(&mut game_state);
            load_level(&game_state.levels[game_state.current_level], &mut commands);
        } else {
            println!("You have completed all levels!");
            // Optionally, trigger a victory screen or reset the game
        }
    }
}





fn despawn_bricks(
    mut commands: Commands,
    brick_query: Query<Entity, With<Brick>>,
) {
    for entity in &brick_query {
        commands.entity(entity).despawn();
    }
}


// Check if the level vector is empty
fn all_bricks_are_cleared(brick_query: Query<Entity, With<Brick>>) -> bool {
    brick_query.is_empty()
}


fn play_collision_sound(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    sound: Res<CollisionSound>,
) {
    // Play a sound once per frame if a collision occurred.
    if !collision_events.is_empty() {
        // This prevents events staying active on the next frame.
        collision_events.clear();
        commands.spawn(AudioBundle {
            source: sound.clone(),
            // auto-despawn the entity when playback finishes
            settings: PlaybackSettings::DESPAWN,
        });
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum Collision {
    Left,
    Right,
    Top,
    Bottom,
}

// Returns `Some` if `ball` collides with `bounding_box`.
// The returned `Collision` is the side of `bounding_box` that `ball` hit.
fn ball_collision(ball: BoundingCircle, bounding_box: Aabb2d) -> Option<Collision> {
    if !ball.intersects(&bounding_box) {
        return None;
    }

    let closest = bounding_box.closest_point(ball.center());
    let offset = ball.center() - closest;
    let side = if offset.x.abs() > offset.y.abs() {
        if offset.x < 0. {
            Collision::Left
        } else {
            Collision::Right
        }
    } else if offset.y > 0. {
        Collision::Top
    } else {
        Collision::Bottom
    };

    Some(side)
}
