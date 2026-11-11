use super::scene::dune_height;
use super::*;
use crate::ship::ARWING_SPEC;

#[test]
fn pause_freezes_mission_time() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.phase = Phase::Playing;
    game.elapsed = 24.0;
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
}

#[test]
fn mission_starts_with_twenty_seconds() {
    let game = HourglassRescue::new();
    assert_eq!(game.remaining(), 20.0);
}

#[test]
fn clocks_form_a_reachable_route_through_the_valley() {
    let start = Vec3::new(0.0, 12.0, 42.0);
    let first_offset = CLOCK_POSITIONS[0].sub(start);
    assert!(
        first_offset.dot(first_offset).sqrt() < 90.0,
        "the first clock must be reachable well before the initial timer expires"
    );

    for (index, pair) in CLOCK_POSITIONS.windows(2).enumerate() {
        let step = pair[1].sub(pair[0]);
        let distance = step.dot(step).sqrt();
        assert!(
            (100.0..=320.0).contains(&distance),
            "clock gap {} should be challenging but reachable, got {distance}",
            index + 1
        );
        assert!(
            pair[1].z < pair[0].z,
            "clocks should advance toward the exit"
        );
        assert!(
            pair[1].y >= pair[0].y,
            "clock height should follow the rising route"
        );
        assert!(
            pair[0].x.signum() != pair[1].x.signum(),
            "clocks should alternate across the exploration valley"
        );
    }
}

#[test]
fn every_energy_fragment_is_available_from_the_start() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    ship.position = CAPSULE_POSITIONS[4];
    game.update_rescue(&ship);
    assert_eq!(game.rescued, 1);
    assert_eq!(game.capsules[4], CapsuleState::Rescued);
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
fn forward_input_moves_toward_the_first_capsule() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.place_ship_at_start(&mut ship);
    game.phase = Phase::Playing;
    let distance_before = ship
        .position
        .sub(CAPSULE_POSITIONS[0])
        .dot(ship.position.sub(CAPSULE_POSITIONS[0]));

    game.update(
        &mut ship,
        Input {
            forward: true,
            ..Input::default()
        },
        0.05,
    );

    let offset_after = ship.position.sub(CAPSULE_POSITIONS[0]);
    let distance_after = offset_after.dot(offset_after);
    assert!(
        ship.position.z < 42.0,
        "forward should move along the chamber toward -Z, got {:?}",
        ship.position
    );
    assert!(
        distance_after < distance_before,
        "the first capsule should get closer"
    );
}

#[test]
fn dune_terrain_is_a_wide_non_degenerate_grid() {
    let game = HourglassRescue::new();
    let range = game.scene.sand_vertices.clone();
    let sand_faces = game
        .scene
        .model
        .faces
        .iter()
        .filter(|face| face.group == "sand_surface");
    let mut count = 0;
    for face in sand_faces {
        count += 1;
        assert_eq!(
            face.shader,
            crate::obj_loader::SurfaceShader::TimePlanetSandTexture
        );
        assert!(face.indices.iter().all(|&i| range.contains(&i)));
        assert_ne!(face.indices[1], face.indices[2]);
        assert_ne!(face.indices[0], face.indices[1]);
        assert_ne!(face.indices[0], face.indices[2]);
    }
    assert!(
        count > 3000,
        "the valley floor should extend beyond the camera view"
    );
}

#[test]
fn rocky_surfaces_use_the_png_stone_shader() {
    let game = HourglassRescue::new();
    let textured_faces = game
        .scene
        .model
        .faces
        .iter()
        .filter(|face| face.shader == crate::obj_loader::SurfaceShader::TimePlanetRockTexture)
        .count();
    assert!(
        textured_faces > 250,
        "mountains, ruins and floating rocks should use the PNG stone texture"
    );
}

#[test]
fn static_sand_keeps_pending_fragments_visible() {
    let mut game = HourglassRescue::new();
    game.update_capsule_visibility();
    let first_face = game.scene.capsule_faces[0].start;
    assert_ne!(game.scene.model.faces[first_face].indices, [0, 0, 0]);
}

#[test]
fn energy_pickups_use_the_imported_blender_fragment() {
    let game = HourglassRescue::new();
    for range in &game.scene.capsule_faces {
        let faces = &game.scene.model.faces[range.clone()];
        assert!(
            faces
                .iter()
                .any(|face| face.group == "Cristal_Central_Superior")
        );
        assert!(
            faces
                .iter()
                .any(|face| face.group == "Aro_Temporal_Exterior")
        );
    }
}

#[test]
fn time_planet_uses_three_distinct_imported_enemy_models() {
    let game = HourglassRescue::new();
    for group in [
        "CHRONOGUARD_Body",
        "PENDULUM_REAPER_Core",
        "HOURGLASS_WRAITH_Top_Rim",
    ] {
        assert!(
            game.scene
                .model
                .faces
                .iter()
                .any(|face| face.group == group),
            "missing imported enemy group {group}"
        );
    }
}

#[test]
fn time_enemy_chases_attacks_and_can_be_destroyed() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&crate::ship::ARWING_SPEC);
    game.phase = Phase::Playing;
    ship.position = game.enemies[0].position.add(Vec3::new(20.0, 0.0, 0.0));
    let old_distance = ship
        .position
        .sub(game.enemies[0].position)
        .dot(ship.position.sub(game.enemies[0].position));
    game.update_enemies(&mut ship, &mut [], 0.05);
    let new_distance = ship
        .position
        .sub(game.enemies[0].position)
        .dot(ship.position.sub(game.enemies[0].position));
    assert!(new_distance < old_distance);
    for _ in 0..15 {
        game.update_enemies(&mut ship, &mut [], 0.05);
    }
    assert!(!game.enemy_projectiles.is_empty());

    ship.position = game.enemies[0].position;
    game.enemy_projectiles.clear();
    game.hit_cooldown = 0.0;
    let shields = game.shields;
    game.update_enemies(&mut ship, &mut [], 0.05);
    assert_eq!(game.shields, shields - 1);

    let mut charged_shot = [crate::ship::Projectile {
        position: game.enemies[0].position,
        direction: Vec3::new(1.0, 0.0, 0.0),
        lifetime: 1.0,
        scale: 4.0,
    }];
    game.update_enemies(&mut ship, &mut charged_shot, 0.05);
    assert!(!game.enemies[0].alive);
    assert_eq!(game.enemies_destroyed, 1);
    assert_eq!(charged_shot[0].lifetime, 0.0);
}

#[test]
fn time_planet_boost_never_runs_out() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&crate::ship::ARWING_SPEC);
    game.place_ship_at_start(&mut ship);
    for _ in 0..200 {
        game.move_ship(
            &mut ship,
            Input {
                boost: true,
                ..Input::default()
            },
            0.05,
        );
    }
    assert_eq!(game.boost_reserve, 2.2);
}

#[test]
fn five_star_reward_plays_race_style_animation_before_exit() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.phase = Phase::Playing;
    game.rescued = CAPSULE_COUNT as u8 - 1;
    game.capsules.fill(CapsuleState::Rescued);
    game.capsules[CAPSULE_COUNT - 1] = CapsuleState::Pending;
    game.elapsed = 0.0;
    ship.position = CAPSULE_POSITIONS[CAPSULE_COUNT - 1];

    assert_eq!(
        game.update(&mut ship, Input::default(), 0.016),
        Action::Continue
    );
    assert_eq!(game.phase, Phase::Reward);
    let frozen = game.reward_position;
    for _ in 0..103 {
        assert_eq!(
            game.update(&mut ship, Input::default(), 0.05),
            Action::Continue
        );
        assert!(ship.position.sub(frozen).dot(ship.position.sub(frozen)) < 0.001);
    }
    assert_eq!(game.update(&mut ship, Input::default(), 0.05), Action::Exit);
    assert_eq!(game.phase, Phase::Won);
}

#[test]
fn colliding_with_fragment_collects_it_once() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.phase = Phase::Playing;
    ship.position = CAPSULE_POSITIONS[0];
    game.update_rescue(&ship);
    game.update_rescue(&ship);
    assert_eq!(game.rescued, 1);
    assert_eq!(game.capsules[0], CapsuleState::Rescued);
}

#[test]
fn clock_pickup_adds_time_and_can_only_be_collected_once() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.phase = Phase::Playing;
    game.elapsed = 30.0;
    ship.position = CLOCK_POSITIONS[0];

    game.update_clocks(&ship);
    game.update_clocks(&ship);

    assert_eq!(game.elapsed, 10.0);
    assert_eq!(game.clocks_collected, 1);
    assert!(game.clocks[0]);
}

#[test]
fn former_portal_location_is_open_space() {
    let game = HourglassRescue::new();
    let approach = Vec3::new(0.0, 155.0, -501.0);
    assert!(game.scene.resolve_obstacle(approach, 3.2).is_none());
}

#[test]
fn five_star_ball_stays_hidden_until_it_is_awarded() {
    let game = HourglassRescue::new();
    assert!(
        game.scene
            .model
            .faces
            .iter()
            .any(|face| face.group == "dragon_star_3")
    );
    assert!(
        game.scene
            .model
            .faces
            .iter()
            .any(|face| face.group == "dragon_star_4")
    );
    assert!(
        game.scene
            .model
            .faces
            .iter()
            .filter(|face| face.group == "dragon_ball")
            .all(|face| face.shader == crate::obj_loader::SurfaceShader::DragonBallGlass)
    );
    assert!(
        game.scene
            .model
            .faces
            .iter()
            .filter(|face| face.group == "dragon_ball" || face.group.starts_with("dragon_star_"))
            .flat_map(|face| face.indices)
            .all(|index| game.scene.model.vertices[index].y < -9_900.0)
    );
}

#[test]
fn one_collision_event_only_removes_one_shield() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    ship.position = Vec3::new(600.0, 20.0, 0.0);
    game.speed = 20.0;
    game.resolve_collisions(&mut ship, 0.05);
    game.resolve_collisions(&mut ship, 0.05);
    assert_eq!(game.shields, 2);
}

#[test]
fn capsule_sites_are_spread_across_the_exploration_valley() {
    let start = Vec3::new(0.0, 12.0, 42.0);
    let first_distance = CAPSULE_POSITIONS[0].sub(start);
    assert!(first_distance.dot(first_distance).sqrt() > 150.0);

    for pair in CAPSULE_POSITIONS.windows(2) {
        let gap = pair[0].sub(pair[1]);
        assert!(gap.dot(gap).sqrt() > 120.0);
    }
    for capsule in CAPSULE_POSITIONS {
        assert!(capsule.x.hypot(capsule.z) < 520.0);
    }
}

#[test]
fn floating_ruins_stop_the_ship_instead_of_being_fly_through_props() {
    let game = HourglassRescue::new();
    let island_center = Vec3::new(-48.0, 75.0, -112.0);
    let resolved = game
        .scene
        .resolve_obstacle(island_center, 3.2)
        .expect("the island needs a gameplay collider");
    let separation = resolved.sub(island_center);
    assert!(separation.dot(separation) > 0.0);
}

#[test]
fn climb_and_descend_inputs_move_in_the_expected_vertical_direction() {
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.place_ship_at_start(&mut ship);
    let start_y = ship.position.y;

    game.move_ship(
        &mut ship,
        Input {
            rise: true,
            ..Input::default()
        },
        0.2,
    );
    assert!(ship.position.y > start_y);

    let raised_y = ship.position.y;
    // Free flight preserves pitch momentum, so reversing direction takes a
    // few frames instead of snapping the ship downward immediately.
    for _ in 0..8 {
        game.move_ship(
            &mut ship,
            Input {
                descend: true,
                ..Input::default()
            },
            0.2,
        );
    }
    assert!(ship.position.y < raised_y);
}

#[test]
fn sand_stream_projection_discards_points_too_close_to_the_camera() {
    let game = HourglassRescue::new();
    let ship = Ship::new(&ARWING_SPEC);
    let camera = game.camera(&ship);
    let near_camera = camera
        .position
        .add(camera.forward.mul(0.21))
        .add(camera.right.mul(1_000.0));
    assert!(super::rendering::project(&camera, near_camera, 1200, 800).is_none());
}

#[test]
fn follow_camera_stays_behind_the_ship_when_it_turns() {
    let game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    ship.position = Vec3::new(10.0, 20.0, 30.0);
    ship.yaw = std::f32::consts::FRAC_PI_2;
    let first = game.camera(&ship);
    let first_forward = ship.movement_basis().0;
    assert!(first.position.sub(ship.position).dot(first_forward) < 0.0);

    ship.yaw = std::f32::consts::PI;
    ship.pitch = 0.35;
    ship.roll = 1.7;
    let second = game.camera(&ship);
    let second_forward = ship.movement_basis().0;
    assert!(second.position.sub(ship.position).dot(second_forward) < 0.0);
    assert!(
        second
            .forward
            .sub(first.forward)
            .dot(second.forward.sub(first.forward))
            > 0.1
    );
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

#[test]
fn write_gameplay_preview_when_capture_path_is_set() {
    let Ok(path) = std::env::var("HOURGLASS_CAPTURE") else {
        return;
    };
    let model = crate::obj_loader::load_obj("assets/vehicles/arwing/arwing2.obj")
        .expect("selected ship preview model should load");
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.place_ship_at_start(&mut ship);
    game.phase = Phase::Playing;
    game.elapsed = 8.0;
    game.rescue_target = Some(0);
    game.sand_offset = 1.6;
    game.scene.update_sand(game.sand_offset);
    game.update_capsule_visibility();
    let mut frame = crate::framebuffer::Framebuffer::new(crate::WIDTH, crate::HEIGHT);
    game.render(&mut frame, &model, &ship);
    game.draw_hud(&mut frame, &ship);
    frame
        .save_png(&path)
        .expect("hourglass rescue preview should be writable");
}

#[test]
fn write_result_preview_when_capture_path_is_set() {
    let Ok(path) = std::env::var("HOURGLASS_RESULT_CAPTURE") else {
        return;
    };
    let model = crate::obj_loader::load_obj("assets/vehicles/arwing/arwing2.obj")
        .expect("selected ship result preview model should load");
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.place_ship_at_start(&mut ship);
    game.phase = Phase::Lost;
    game.elapsed = MISSION_SECONDS;
    game.shields = 1;
    game.rescued = 2;
    for index in 0..2 {
        game.capsules[index] = CapsuleState::Rescued;
        game.scene.hide_capsule(index);
    }
    let mut frame = crate::framebuffer::Framebuffer::new(crate::WIDTH, crate::HEIGHT);
    game.render(&mut frame, &model, &ship);
    game.draw_hud(&mut frame, &ship);
    frame
        .save_png(&path)
        .expect("hourglass rescue result preview should be writable");
}

#[test]
fn write_victory_preview_when_capture_path_is_set() {
    let Ok(path) = std::env::var("HOURGLASS_WIN_CAPTURE") else {
        return;
    };
    let model = crate::obj_loader::load_obj("assets/vehicles/arwing/arwing2.obj")
        .expect("selected ship victory preview model should load");
    let mut game = HourglassRescue::new();
    let mut ship = Ship::new(&ARWING_SPEC);
    game.place_ship_at_start(&mut ship);
    game.phase = Phase::Won;
    game.elapsed = 112.0;
    game.rescued = CAPSULE_COUNT as u8;
    game.capsules.fill(CapsuleState::Rescued);
    for index in 0..CAPSULE_COUNT {
        game.scene.hide_capsule(index);
    }
    let mut frame = crate::framebuffer::Framebuffer::new(crate::WIDTH, crate::HEIGHT);
    game.render(&mut frame, &model, &ship);
    game.draw_hud(&mut frame, &ship);
    frame
        .save_png(&path)
        .expect("hourglass rescue victory preview should be writable");
}
