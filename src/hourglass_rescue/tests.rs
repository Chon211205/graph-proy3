use super::scene::dune_height;
use super::*;
use crate::ship::ARWING_SPEC;

#[test]
fn pause_freezes_mission_and_rescue_progress() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.phase = Phase::Playing;
    game.elapsed = 24.0;
    game.rescue_progress = 0.4;
    game.update(
        &mut ship,
        Input {
            pause: true,
            ..Input::default()
        },
        0.05,
    );
    let elapsed = game.elapsed;
    game.update(&mut ship, Input::default(), 1.0);
    assert_eq!(game.elapsed, elapsed);
    assert_eq!(game.rescue_progress, 0.4);
}

#[test]
fn rescue_target_is_restricted_to_accessible_capsules_before_inversion() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    ship.position = CAPSULE_POSITIONS[4];
    game.update_rescue(&ship, Input::default(), 0.05);
    assert!(game.rescue_target.is_some_and(|index| index < 3));

    game.inversion_complete = true;
    game.update_rescue(&ship, Input::default(), 0.05);
    assert!(game.rescue_target.is_some_and(|index| index >= 3));
}

#[test]
fn sand_collision_height_tracks_visible_surface_offset() {
    let offset = 5.5;
    let mut game = HourglassRescue::new();
    game.scene.update_sand(offset);
    let vertex = game.scene.model.vertices[game.scene.sand_vertices.start];
    let collision_height = -8.0 + dune_height(vertex.x, vertex.z) + offset;
    assert!((vertex.y - collision_height).abs() < 0.001);
}

#[test]
fn rising_sand_hides_and_then_reveals_the_low_capsule() {
    let mut game = HourglassRescue::new();
    game.sand_offset = SAND_RISE;
    assert!(game.capsule_is_buried(0));
    game.update_capsule_visibility();
    let first_face = game.scene.capsule_faces[0].start;
    assert_eq!(game.scene.model.faces[first_face].indices, [0, 0, 0]);

    game.sand_offset = -1.8;
    assert!(!game.capsule_is_buried(0));
    game.update_capsule_visibility();
    assert_ne!(game.scene.model.faces[first_face].indices, [0, 0, 0]);
}

#[test]
fn holding_rescue_collects_a_capsule_once() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.phase = Phase::Playing;
    ship.position = CAPSULE_POSITIONS[0];
    game.update_rescue(
        &ship,
        Input {
            rescue: true,
            ..Input::default()
        },
        RESCUE_SECONDS,
    );
    game.update_rescue(
        &ship,
        Input {
            rescue: true,
            ..Input::default()
        },
        RESCUE_SECONDS,
    );
    assert_eq!(game.rescued, 1);
    assert_eq!(game.capsules[0], CapsuleState::Rescued);
}

#[test]
fn gravity_inversion_opens_the_upper_route_without_adding_mission_time() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.phase = Phase::Warning;
    game.elapsed = INVERSION_AT - INVERSION_WARNING;
    for _ in 0..240 {
        game.update(&mut ship, Input::default(), 0.05);
        if game.inversion_complete {
            break;
        }
    }
    assert!(game.inversion_complete);
    assert_eq!(game.phase, Phase::Playing);
    assert!(game.elapsed <= INVERSION_AT + 0.05);
}

#[test]
fn one_collision_event_only_removes_one_shield() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    ship.position = Vec3::new(100.0, 20.0, 0.0);
    game.speed = 20.0;
    game.resolve_collisions(&mut ship, 0.05);
    game.resolve_collisions(&mut ship, 0.05);
    assert_eq!(game.shields, 2);
}

#[test]
fn retry_resets_capsules_timer_and_defense() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.phase = Phase::Lost;
    game.elapsed = 179.0;
    game.shields = 0;
    game.capsules[0] = CapsuleState::Rescued;
    game.rescued = 1;
    game.update(
        &mut ship,
        Input {
            restart: true,
            ..Input::default()
        },
        0.05,
    );
    assert_eq!(game.phase, Phase::Intro);
    assert_eq!(game.rescued, 0);
    assert_eq!(game.shields, 3);
    assert_eq!(game.elapsed, 0.0);
}
