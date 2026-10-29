use std::f32::consts::{PI, TAU};

use crate::obj_loader::Vec3;

#[derive(Clone, Copy)]
pub enum ModelAxes {
    Arwing,
    Kirby,
    Astra,
    CowboyB,
}

#[derive(Clone, Copy)]
pub struct ShipSpec {
    pub name: &'static str,
    pub model_scale: f32,
    pub model_axes: ModelAxes,
    pub collision_radius: f32,
    pub min_speed: f32,
    pub cruise_speed: f32,
    pub max_speed: f32,
    pub acceleration_rate: f32,
    pub braking_rate: f32,
    pub cruise_recovery_rate: f32,
    pub roll_input_speed: f32,
    pub roll_speed: f32,
    pub max_manual_roll: f32,
    pub auto_level_speed: f32,
    pub turn_response: f32,
    pub max_yaw_rate: f32,
    pub max_pitch_rate: f32,
    pub steering_bank: f32,
    pub steering_pitch: f32,
    pub chase_distance: f32,
    pub chase_height: f32,
    pub chase_look_ahead: f32,
    pub turbo_back: f32,
    pub turbo_side: f32,
    pub turbo_down: f32,
    pub turbo_sides: &'static [f32],
    pub turbo_color: [u8; 3],
    pub turbo_core_color: [u8; 3],
    pub laser_speed: f32,
    pub laser_lifetime: f32,
    pub max_charge_time: f32,
    pub charged_shot_threshold: f32,
    pub muzzle_forward: f32,
    pub muzzle_side: f32,
    pub muzzle_down: f32,
    pub muzzle_sides: &'static [f32],
    pub laser_color: [u8; 3],
    pub laser_core_color: [u8; 3],
}

const DUAL_SIDES: &[f32] = &[-1.0, 1.0];
const CENTER_SIDE: &[f32] = &[0.0];

pub static ARWING_SPEC: ShipSpec = ShipSpec {
    name: "Arwing",
    model_scale: 0.055,
    model_axes: ModelAxes::Arwing,
    collision_radius: 2.0,
    min_speed: 4.0,
    cruise_speed: 12.0,
    max_speed: 55.0,
    acceleration_rate: 20.0,
    braking_rate: 28.0,
    cruise_recovery_rate: 18.0,
    roll_input_speed: PI * 0.5,
    roll_speed: PI,
    max_manual_roll: PI * 0.22,
    auto_level_speed: 3.2,
    turn_response: 7.0,
    max_yaw_rate: 1.15,
    max_pitch_rate: 1.1,
    steering_bank: 0.48,
    steering_pitch: 0.24,
    chase_distance: 12.0,
    chase_height: 4.0,
    chase_look_ahead: 8.0,
    turbo_back: 1.12,
    turbo_side: 0.72,
    turbo_down: 0.05,
    turbo_sides: DUAL_SIDES,
    turbo_color: [36, 142, 255],
    turbo_core_color: [220, 252, 255],
    laser_speed: 220.0,
    laser_lifetime: 3.0,
    max_charge_time: 1.4,
    charged_shot_threshold: 0.35,
    muzzle_forward: 1.6,
    muzzle_side: 1.15,
    muzzle_down: 0.05,
    muzzle_sides: DUAL_SIDES,
    laser_color: [255, 48, 38],
    laser_core_color: [255, 245, 190],
};

pub static KIRBY_SPEC: ShipSpec = ShipSpec {
    name: "Kirby UFO",
    model_scale: 0.25,
    model_axes: ModelAxes::Kirby,
    collision_radius: 2.0,
    min_speed: 4.0,
    cruise_speed: 12.0,
    max_speed: 55.0,
    acceleration_rate: 20.0,
    braking_rate: 28.0,
    cruise_recovery_rate: 18.0,
    roll_input_speed: PI * 0.5,
    roll_speed: PI,
    max_manual_roll: PI * 0.22,
    auto_level_speed: 3.2,
    turn_response: 7.0,
    max_yaw_rate: 1.15,
    max_pitch_rate: 1.1,
    steering_bank: 0.38,
    steering_pitch: 0.2,
    chase_distance: 12.0,
    chase_height: 4.0,
    chase_look_ahead: 8.0,
    turbo_back: 1.45,
    turbo_side: 0.0,
    turbo_down: 0.2,
    turbo_sides: CENTER_SIDE,
    turbo_color: [255, 190, 18],
    turbo_core_color: [255, 252, 180],
    laser_speed: 220.0,
    laser_lifetime: 3.0,
    max_charge_time: 1.4,
    charged_shot_threshold: 0.35,
    muzzle_forward: 1.75,
    muzzle_side: 0.82,
    muzzle_down: 0.0,
    muzzle_sides: DUAL_SIDES,
    laser_color: [38, 255, 92],
    laser_core_color: [205, 255, 214],
};

pub static ASTRA_SPEC: ShipSpec = ShipSpec {
    name: "ASTRA",
    model_scale: 0.23,
    model_axes: ModelAxes::Astra,
    collision_radius: 2.0,
    min_speed: 4.0,
    cruise_speed: 12.0,
    max_speed: 55.0,
    acceleration_rate: 20.0,
    braking_rate: 28.0,
    cruise_recovery_rate: 18.0,
    roll_input_speed: PI * 0.5,
    roll_speed: PI,
    max_manual_roll: PI * 0.22,
    auto_level_speed: 3.2,
    turn_response: 7.0,
    max_yaw_rate: 1.15,
    max_pitch_rate: 1.1,
    steering_bank: 0.46,
    steering_pitch: 0.22,
    chase_distance: 12.0,
    chase_height: 4.0,
    chase_look_ahead: 8.0,
    turbo_back: 1.18,
    turbo_side: 0.72,
    turbo_down: 0.05,
    turbo_sides: DUAL_SIDES,
    turbo_color: [35, 145, 255],
    turbo_core_color: [120, 235, 255],
    laser_speed: 220.0,
    laser_lifetime: 3.0,
    max_charge_time: 1.4,
    charged_shot_threshold: 0.35,
    muzzle_forward: 1.6,
    muzzle_side: 1.05,
    muzzle_down: 0.0,
    muzzle_sides: DUAL_SIDES,
    laser_color: [35, 145, 255],
    laser_core_color: [190, 245, 255],
};

pub static COWBOYB_SPEC: ShipSpec = ShipSpec {
    name: "Cowboy B",
    model_scale: 0.28,
    model_axes: ModelAxes::CowboyB,
    collision_radius: 2.0,
    min_speed: 4.0,
    cruise_speed: 12.0,
    max_speed: 55.0,
    acceleration_rate: 20.0,
    braking_rate: 28.0,
    cruise_recovery_rate: 18.0,
    roll_input_speed: PI * 0.5,
    roll_speed: PI,
    max_manual_roll: PI * 0.22,
    auto_level_speed: 3.2,
    turn_response: 7.0,
    max_yaw_rate: 1.15,
    max_pitch_rate: 1.1,
    steering_bank: 0.42,
    steering_pitch: 0.21,
    chase_distance: 12.0,
    chase_height: 4.0,
    chase_look_ahead: 8.0,
    turbo_back: 1.1,
    turbo_side: 0.62,
    turbo_down: 0.05,
    turbo_sides: DUAL_SIDES,
    turbo_color: [255, 105, 35],
    turbo_core_color: [255, 230, 155],
    laser_speed: 220.0,
    laser_lifetime: 3.0,
    max_charge_time: 1.4,
    charged_shot_threshold: 0.35,
    muzzle_forward: 1.55,
    muzzle_side: 0.82,
    muzzle_down: 0.0,
    muzzle_sides: DUAL_SIDES,
    laser_color: [255, 55, 45],
    laser_core_color: [255, 225, 170],
};

pub static BUZZ_SPEC: ShipSpec = ShipSpec {
    name: "Buzz Lightyear",
    model_scale: 0.30,
    model_axes: ModelAxes::Astra,
    turbo_color: [144, 240, 40],
    turbo_core_color: [230, 255, 200],
    laser_color: [255, 45, 55],
    laser_core_color: [255, 220, 230],
    ..ASTRA_SPEC
};

impl ShipSpec {
    pub fn model_to_world(&self, local: Vec3, forward: Vec3, right: Vec3, up: Vec3) -> Vec3 {
        match self.model_axes {
            ModelAxes::Arwing => forward
                .mul(-local.x)
                .add(up.mul(local.y))
                .add(right.mul(local.z)),
            ModelAxes::Kirby => forward
                .mul(-local.y)
                .add(up.mul(local.z))
                .add(right.mul(local.x)),
            ModelAxes::Astra => forward
                .mul(local.z)
                .add(up.mul(local.y))
                .add(right.mul(local.x)),
            ModelAxes::CowboyB => forward
                .mul(-local.y)
                .add(up.mul(local.z))
                .add(right.mul(local.x)),
        }
    }
}

#[derive(Default)]
pub struct ShipInput {
    pub pitch: i8,
    pub yaw: i8,
    pub roll: i8,
    pub accelerating: bool,
    pub braking: bool,
    pub firing: bool,
}

pub struct Projectile {
    pub position: Vec3,
    pub direction: Vec3,
    pub lifetime: f32,
    pub scale: f32,
}

pub struct Ship {
    pub spec: &'static ShipSpec,
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub roll_target: f32,
    pub yaw_rate: f32,
    pub pitch_rate: f32,
    pub steering_bank: f32,
    pub steering_pitch: f32,
    pub orbit_axis: Option<Vec3>,
    pub orbit_center: Option<Vec3>,
    pub orbit_camera_blend: f32,
    pub planet_locked: bool,
    pub orbit_release_active: bool,
    pub speed: f32,
}

impl Ship {
    pub fn new(spec: &'static ShipSpec) -> Self {
        Self {
            spec,
            position: Vec3::new(0.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            roll_target: 0.0,
            yaw_rate: 0.0,
            pitch_rate: 0.0,
            steering_bank: 0.0,
            steering_pitch: 0.0,
            orbit_axis: None,
            orbit_center: None,
            orbit_camera_blend: 0.0,
            planet_locked: false,
            orbit_release_active: false,
            speed: spec.cruise_speed,
        }
    }

    pub fn update_flight(&mut self, input: &ShipInput, delta_time: f32) {
        let response = 1.0 - (-self.spec.turn_response * delta_time).exp();
        let target_pitch_rate = input.pitch as f32 * self.spec.max_pitch_rate;
        let target_yaw_rate = input.yaw as f32 * self.spec.max_yaw_rate;
        self.pitch_rate += (target_pitch_rate - self.pitch_rate) * response;
        self.yaw_rate += (target_yaw_rate - self.yaw_rate) * response;
        self.pitch = (self.pitch + self.pitch_rate * delta_time).clamp(-1.25, 1.25);
        self.yaw = (self.yaw + self.yaw_rate * delta_time).rem_euclid(TAU);

        let target_bank = -(input.yaw as f32) * self.spec.steering_bank;
        self.steering_bank += (target_bank - self.steering_bank) * response;
        let target_visual_pitch = input.pitch as f32 * self.spec.steering_pitch;
        self.steering_pitch += (target_visual_pitch - self.steering_pitch) * response;

        if self.planet_locked {
            if input.roll != 0 {
                self.roll_target = (self.roll_target
                    + input.roll as f32 * self.spec.roll_input_speed * delta_time)
                    .clamp(-self.spec.max_manual_roll, self.spec.max_manual_roll);
            } else {
                let level_response = 1.0 - (-self.spec.auto_level_speed * delta_time).exp();
                self.roll_target += (0.0 - self.roll_target) * level_response;
            }
        } else {
            self.roll_target += input.roll as f32 * self.spec.roll_input_speed * delta_time;
        }
        let roll_difference = self.roll_target - self.roll;
        let roll_step = (self.spec.roll_speed * delta_time).min(roll_difference.abs());
        self.roll += roll_difference.signum() * roll_step;

        let (target_speed, rate) = match (input.accelerating, input.braking) {
            (true, false) => (self.spec.max_speed, self.spec.acceleration_rate),
            (false, true) => (self.spec.min_speed, self.spec.braking_rate),
            _ => (self.spec.cruise_speed, self.spec.cruise_recovery_rate),
        };
        let difference = target_speed - self.speed;
        let step = (rate * delta_time).min(difference.abs());
        self.speed += difference.signum() * step;
    }

    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        self.basis_with_angles(
            self.pitch + self.steering_pitch,
            self.roll + self.steering_bank,
        )
    }

    pub fn camera_basis(&self) -> (Vec3, Vec3, Vec3) {
        self.basis_with_angles(self.pitch, self.roll)
    }

    pub fn movement_basis(&self) -> (Vec3, Vec3, Vec3) {
        self.basis_with_angles(self.pitch, self.roll)
    }

    fn basis_with_angles(&self, pitch: f32, roll: f32) -> (Vec3, Vec3, Vec3) {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = pitch.sin_cos();
        let forward = Vec3::new(-cos_pitch * cos_yaw, sin_pitch, -cos_pitch * sin_yaw).normalize();
        let level_right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
        let level_up = level_right.cross(forward).normalize();
        let (sin_roll, cos_roll) = roll.sin_cos();
        let right = level_right.mul(cos_roll).add(level_up.mul(sin_roll));
        let up = level_up.mul(cos_roll).sub(level_right.mul(sin_roll));
        (forward, right, up)
    }
}

pub struct ShipController {
    pub ship: Ship,
    pub projectiles: Vec<Projectile>,
    fire_charge: f32,
    fire_was_down: bool,
}

impl ShipController {
    pub fn new(spec: &'static ShipSpec) -> Self {
        Self {
            ship: Ship::new(spec),
            projectiles: Vec::new(),
            fire_charge: 0.0,
            fire_was_down: false,
        }
    }

    pub fn reset(&mut self) {
        let spec = self.ship.spec;
        *self = Self::new(spec);
    }

    pub fn update(&mut self, input: &ShipInput, delta_time: f32) {
        self.ship.update_flight(input, delta_time);
        self.update_weapon(input.firing, delta_time);
        for projectile in &mut self.projectiles {
            projectile.position = projectile.position.add(
                projectile
                    .direction
                    .mul(self.ship.spec.laser_speed * delta_time),
            );
            projectile.lifetime -= delta_time;
        }
        self.projectiles
            .retain(|projectile| projectile.lifetime > 0.0);
    }

    fn update_weapon(&mut self, firing: bool, delta_time: f32) {
        if firing {
            self.fire_charge = (self.fire_charge + delta_time).min(self.ship.spec.max_charge_time);
        } else if self.fire_was_down {
            let (forward, right, up) = self.ship.basis();
            if self.fire_charge >= self.ship.spec.charged_shot_threshold {
                let charge = self.charge_ratio();
                self.projectiles.push(Projectile {
                    position: self
                        .ship
                        .position
                        .add(forward.mul(self.ship.spec.muzzle_forward + 0.4)),
                    direction: forward,
                    lifetime: self.ship.spec.laser_lifetime,
                    scale: 1.5 + charge * 2.5,
                });
            } else {
                for side in self.ship.spec.muzzle_sides {
                    self.projectiles.push(Projectile {
                        position: self
                            .ship
                            .position
                            .add(forward.mul(self.ship.spec.muzzle_forward))
                            .add(right.mul(*side * self.ship.spec.muzzle_side))
                            .sub(up.mul(self.ship.spec.muzzle_down)),
                        direction: forward,
                        lifetime: self.ship.spec.laser_lifetime,
                        scale: 1.0,
                    });
                }
            }
            self.fire_charge = 0.0;
        }
        self.fire_was_down = firing;
    }

    pub fn charge_ratio(&self) -> f32 {
        (self.fire_charge / self.ship.spec.max_charge_time).clamp(0.0, 1.0)
    }

    pub fn turbo_power(&self, active_speed: f32) -> f32 {
        ((active_speed - self.ship.spec.min_speed)
            / (self.ship.spec.max_speed - self.ship.spec.min_speed))
            .clamp(0.0, 1.0)
    }
}
