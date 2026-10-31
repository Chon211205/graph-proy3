mod visuals;

const STARTING_SHIELDS: u8 = 3;
const BOSS_HEALTH: u32 = 42;
const PLAYER_SPEED: f32 = 0.82;
const PLAYER_SHOT_INTERVAL: f32 = 0.45;
const PLAYER_SHOT_SPEED: f32 = 1.35;
const SEED_SPEED: f32 = 0.42;
const INVULNERABILITY_TIME: f32 = 1.25;
const MAX_PLAYER_SHOTS: usize = 28;
const MAX_SEEDS: usize = 12;
const MAX_DROPS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Arrival,
    Closed,
    Warning,
    Exposed,
    Closing,
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GuardianInput {
    pub horizontal: f32,
    pub vertical: f32,
    pub fire: bool,
    pub confirm: bool,
    pub exit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardianAction {
    Stay,
    Exit,
}

#[derive(Clone, Copy)]
struct PlayerShot {
    x: f32,
    y: f32,
    previous_x: f32,
    previous_y: f32,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Seed {
    x: f32,
    y: f32,
    previous_x: f32,
    previous_y: f32,
    velocity_x: f32,
    velocity_y: f32,
    alive: bool,
}

#[derive(Clone, Copy)]
struct ShieldDrop {
    x: f32,
    y: f32,
    alive: bool,
}

pub struct CoreGuardian {
    phase: Phase,
    phase_time: f32,
    elapsed: f32,
    boss_health: u32,
    shields: u8,
    invulnerability: f32,
    player_x: f32,
    player_y: f32,
    previous_player_x: f32,
    previous_player_y: f32,
    velocity_x: f32,
    velocity_y: f32,
    roll: f32,
    fire_timer: f32,
    attack_timer: f32,
    open_plate: usize,
    cycle: u32,
    hit_flash: f32,
    random: u32,
    player_shots: Vec<PlayerShot>,
    seeds: Vec<Seed>,
    shield_drops: Vec<ShieldDrop>,
}

impl CoreGuardian {
    pub fn new() -> Self {
        Self {
            phase: Phase::Arrival,
            phase_time: 0.0,
            elapsed: 0.0,
            boss_health: BOSS_HEALTH,
            shields: STARTING_SHIELDS,
            invulnerability: 0.0,
            player_x: 0.5,
            player_y: 0.0,
            previous_player_x: 0.5,
            previous_player_y: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            roll: 0.0,
            fire_timer: 0.0,
            attack_timer: 1.0,
            open_plate: 0,
            cycle: 0,
            hit_flash: 0.0,
            random: 0x91e1_0da5,
            player_shots: Vec::with_capacity(MAX_PLAYER_SHOTS),
            seeds: Vec::with_capacity(MAX_SEEDS),
            shield_drops: Vec::with_capacity(MAX_DROPS),
        }
    }

    pub fn update(
        &mut self,
        delta_time: f32,
        input: GuardianInput,
        width: usize,
        height: usize,
    ) -> GuardianAction {
        let dt = delta_time.clamp(0.0, 0.05);
        self.elapsed += dt;
        self.phase_time += dt;
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        self.invulnerability = (self.invulnerability - dt).max(0.0);

        if input.exit {
            return GuardianAction::Exit;
        }
        if matches!(self.phase, Phase::Won | Phase::Lost) {
            if input.confirm && self.phase == Phase::Lost {
                *self = Self::new();
            } else if input.confirm && self.phase == Phase::Won {
                return GuardianAction::Exit;
            }
            return GuardianAction::Stay;
        }

        let response = 1.0 - (-10.0 * dt).exp();
        self.previous_player_x = self.player_x;
        self.previous_player_y = self.player_y;
        let horizontal = input.horizontal.clamp(-1.0, 1.0);
        let vertical = input.vertical.clamp(-1.0, 1.0);
        self.velocity_x += (horizontal * PLAYER_SPEED - self.velocity_x) * response;
        self.velocity_y += (vertical * PLAYER_SPEED - self.velocity_y) * response;
        self.player_x = (self.player_x + self.velocity_x * dt).clamp(0.12, 0.88);
        self.player_y = (self.player_y + self.velocity_y * dt).clamp(-0.72, 0.68);
        self.roll += ((-horizontal * 0.22) - self.roll) * (1.0 - (-7.0 * dt).exp());

        match self.phase {
            Phase::Arrival if self.phase_time >= 1.6 => self.set_phase(Phase::Closed),
            Phase::Closed => {
                self.attack_timer -= dt;
                let interval = if self.boss_health * 2 <= BOSS_HEALTH {
                    0.9
                } else {
                    1.45
                };
                if self.attack_timer <= 0.0 && self.seeds.len() < MAX_SEEDS {
                    self.spawn_seed(width, height);
                    self.attack_timer += interval;
                }
                if self.phase_time >= 4.1 {
                    self.open_plate = self.cycle as usize % 3;
                    self.set_phase(Phase::Warning);
                }
            }
            Phase::Warning if self.phase_time >= 0.85 => self.set_phase(Phase::Exposed),
            Phase::Exposed if self.phase_time >= 2.35 => self.set_phase(Phase::Closing),
            Phase::Closing if self.phase_time >= 0.65 => {
                self.cycle = self.cycle.wrapping_add(1);
                self.set_phase(Phase::Closed);
            }
            _ => {}
        }

        self.update_seeds(dt, width, height);
        self.update_player_shots(dt, input.fire, width, height);
        self.update_drops(dt, width, height);
        GuardianAction::Stay
    }

    fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.phase_time = 0.0;
        if phase == Phase::Closed {
            self.attack_timer = self.attack_timer.max(0.65);
        }
    }

    fn boss_center(&self) -> (f32, f32) {
        (
            0.5 + (self.elapsed * 0.48).sin() * 0.035,
            0.335 + (self.elapsed * 0.8).sin() * 0.012,
        )
    }

    fn spawn_seed(&mut self, width: usize, height: usize) {
        let (boss_x, boss_y) = self.boss_center();
        let lane = (self.next_random() % 5) as f32 - 2.0;
        let seed_x = (boss_x + lane * 0.04).clamp(0.18, 0.82);
        let seed_y = boss_y + 0.055;
        let target_x = self.player_x;
        let target_y = 0.82 + self.player_y * 0.10;
        let dx = (target_x - seed_x) * width as f32;
        let dy = (target_y - seed_y) * height as f32;
        let length = (dx * dx + dy * dy).sqrt().max(1.0);
        let speed = height as f32 * SEED_SPEED;
        let seed = Seed {
            x: seed_x,
            y: seed_y,
            previous_x: seed_x,
            previous_y: seed_y,
            velocity_x: dx / length * speed / width.max(1) as f32,
            velocity_y: dy / length * speed / height.max(1) as f32,
            alive: true,
        };
        self.put_seed(seed);
    }

    fn put_seed(&mut self, seed: Seed) {
        if let Some(slot) = self.seeds.iter_mut().find(|item| !item.alive) {
            *slot = seed;
        } else if self.seeds.len() < MAX_SEEDS {
            self.seeds.push(seed);
        }
    }

    fn next_random(&mut self) -> u32 {
        let mut value = self.random;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.random = value;
        value
    }

    fn update_player_shots(&mut self, dt: f32, fire: bool, width: usize, height: usize) {
        self.fire_timer -= dt;
        if fire && self.fire_timer <= 0.0 {
            let shot = PlayerShot {
                x: self.player_x,
                y: 0.82 + self.player_y * 0.10,
                previous_x: self.player_x,
                previous_y: 0.82 + self.player_y * 0.10,
                alive: true,
            };
            if let Some(slot) = self.player_shots.iter_mut().find(|item| !item.alive) {
                *slot = shot;
            } else if self.player_shots.len() < MAX_PLAYER_SHOTS {
                self.player_shots.push(shot);
            }
            self.fire_timer = PLAYER_SHOT_INTERVAL;
        }

        let (boss_x, boss_y) = self.boss_center();
        let core_radius = 0.052;
        let mut boss_defeated = false;
        for shot in self.player_shots.iter_mut().filter(|shot| shot.alive) {
            shot.previous_x = shot.x;
            shot.previous_y = shot.y;
            shot.y -= PLAYER_SHOT_SPEED * dt;
            let shot_hit_boss = shot.y <= boss_y + 0.10
                && shot.y >= boss_y - 0.06
                && (shot.x - boss_x).abs() <= core_radius;
            if shot_hit_boss {
                shot.alive = false;
                self.hit_flash = 0.16;
                if self.phase == Phase::Exposed {
                    self.boss_health = self.boss_health.saturating_sub(1);
                    if self.boss_health == 0 {
                        boss_defeated = true;
                    }
                }
            } else if shot.y < 0.08 {
                shot.alive = false;
            }
        }

        let mut drops = Vec::new();
        for seed in self.seeds.iter_mut().filter(|seed| seed.alive) {
            for shot in self.player_shots.iter_mut().filter(|shot| shot.alive) {
                if swept_distance(
                    (
                        shot.previous_x * width as f32,
                        shot.previous_y * height as f32,
                    ),
                    (shot.x * width as f32, shot.y * height as f32),
                    (
                        seed.previous_x * width as f32,
                        seed.previous_y * height as f32,
                    ),
                    (seed.x * width as f32, seed.y * height as f32),
                ) < 15.0
                {
                    shot.alive = false;
                    seed.alive = false;
                    drops.push((seed.x, seed.y));
                    break;
                }
            }
        }
        for (x, y) in drops {
            self.spawn_shield_drop(x, y);
        }
        if boss_defeated {
            self.set_phase(Phase::Won);
        }
    }

    fn spawn_shield_drop(&mut self, x: f32, y: f32) {
        if self.next_random() % 4 != 0 {
            return;
        }
        let drop = ShieldDrop { x, y, alive: true };
        if let Some(slot) = self.shield_drops.iter_mut().find(|item| !item.alive) {
            *slot = drop;
        } else if self.shield_drops.len() < MAX_DROPS {
            self.shield_drops.push(drop);
        }
    }

    fn update_seeds(&mut self, dt: f32, width: usize, height: usize) {
        let player_x = self.player_x;
        let player_y = 0.82 + self.player_y * 0.10;
        let previous_player_x = self.previous_player_x;
        let previous_player_y = 0.82 + self.previous_player_y * 0.10;
        let mut lost = false;
        for seed in self.seeds.iter_mut().filter(|seed| seed.alive) {
            seed.previous_x = seed.x;
            seed.previous_y = seed.y;
            seed.x += seed.velocity_x * dt;
            seed.y += seed.velocity_y * dt;
            let hit = swept_distance(
                (
                    seed.previous_x * width as f32,
                    seed.previous_y * height as f32,
                ),
                (seed.x * width as f32, seed.y * height as f32),
                (
                    previous_player_x * width as f32,
                    previous_player_y * height as f32,
                ),
                (player_x * width as f32, player_y * height as f32),
            ) < 28.0;
            if hit && self.invulnerability <= 0.0 {
                seed.alive = false;
                self.shields = self.shields.saturating_sub(1);
                self.invulnerability = INVULNERABILITY_TIME;
                if self.shields == 0 {
                    lost = true;
                }
            } else if seed.y > 1.08 || seed.x < -0.1 || seed.x > 1.1 {
                seed.alive = false;
            }
        }
        if lost {
            self.set_phase(Phase::Lost);
        }
    }

    fn update_drops(&mut self, dt: f32, width: usize, height: usize) {
        let player_x = self.player_x;
        let player_y = 0.82 + self.player_y * 0.10;
        for drop in self.shield_drops.iter_mut().filter(|drop| drop.alive) {
            drop.y += 0.16 * dt;
            let dx = (drop.x - player_x) * width as f32;
            let dy = (drop.y - player_y) * height as f32;
            if dx * dx + dy * dy < 34.0 * 34.0 {
                self.shields = (self.shields + 1).min(STARTING_SHIELDS);
                drop.alive = false;
            } else if drop.y > 1.1 {
                drop.alive = false;
            }
        }
    }

    fn opening_amount(&self, plate: usize) -> f32 {
        if plate != self.open_plate {
            return 0.0;
        }
        match self.phase {
            Phase::Warning => (self.phase_time / 0.85).clamp(0.0, 1.0) * 0.22,
            Phase::Exposed => 1.0,
            Phase::Closing => (1.0 - self.phase_time / 0.65).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }

    pub fn is_victory(&self) -> bool {
        self.phase == Phase::Won
    }

    pub fn is_defeat(&self) -> bool {
        self.phase == Phase::Lost
    }

    pub fn health_ratio(&self) -> f32 {
        self.boss_health as f32 / BOSS_HEALTH as f32
    }

    pub fn is_exposed(&self) -> bool {
        self.phase == Phase::Exposed
    }
}

fn swept_distance(a0: (f32, f32), a1: (f32, f32), b0: (f32, f32), b1: (f32, f32)) -> f32 {
    let start = (a0.0 - b0.0, a0.1 - b0.1);
    let velocity = ((a1.0 - a0.0) - (b1.0 - b0.0), (a1.1 - a0.1) - (b1.1 - b0.1));
    let length_sq = velocity.0 * velocity.0 + velocity.1 * velocity.1;
    let t = if length_sq > 0.0001 {
        (-(start.0 * velocity.0 + start.1 * velocity.1) / length_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let dx = start.0 + velocity.0 * t;
    let dy = start.1 + velocity.1 * t;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_shell_blocks_player_shots() {
        let mut fight = CoreGuardian::new();
        fight.set_phase(Phase::Closed);
        for _ in 0..16 {
            fight.update(
                0.05,
                GuardianInput {
                    fire: true,
                    ..GuardianInput::default()
                },
                1200,
                800,
            );
        }
        assert_eq!(fight.boss_health, BOSS_HEALTH);
    }

    #[test]
    fn exposed_core_takes_damage_from_player_shots() {
        let mut fight = CoreGuardian::new();
        fight.set_phase(Phase::Exposed);
        for _ in 0..12 {
            fight.update(
                0.05,
                GuardianInput {
                    fire: true,
                    ..GuardianInput::default()
                },
                1200,
                800,
            );
        }
        assert!(fight.boss_health < BOSS_HEALTH);
    }

    #[test]
    fn seed_keeps_its_launch_direction() {
        let mut fight = CoreGuardian::new();
        fight.player_x = 0.75;
        fight.spawn_seed(1200, 800);
        let direction = (fight.seeds[0].velocity_x, fight.seeds[0].velocity_y);
        fight.player_x = 0.18;
        fight.update_seeds(0.1, 1200, 800);
        assert_eq!(
            (fight.seeds[0].velocity_x, fight.seeds[0].velocity_y),
            direction
        );
    }

    #[test]
    fn swept_collision_detects_fast_crossing_projectiles() {
        let distance = swept_distance((20.0, 0.0), (20.0, 100.0), (0.0, 50.0), (40.0, 50.0));
        assert!(distance < 0.01);
    }

    #[test]
    fn retry_restores_health_and_three_shields() {
        let mut fight = CoreGuardian::new();
        fight.phase = Phase::Lost;
        fight.shields = 0;
        fight.boss_health = 9;
        fight.update(
            0.016,
            GuardianInput {
                confirm: true,
                ..GuardianInput::default()
            },
            1200,
            800,
        );
        assert_eq!(fight.phase, Phase::Arrival);
        assert_eq!(fight.shields, STARTING_SHIELDS);
        assert_eq!(fight.boss_health, BOSS_HEALTH);
    }
}
