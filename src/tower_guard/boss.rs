mod hat;
mod rocket;

use super::{config::EnemyKind, state::*};

pub const BOSS_MAGAZINE: u8 = 6;
pub const BOSS_RELOAD_SECONDS: f32 = 1.05;
pub const BOSS_FIRE_INTERVAL: f32 = 0.27;
pub const HAT_MAX_HEALTH: i32 = 24;
pub const ROCKET_MAX_HEALTH: i32 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossHit {
    WeakPoint,
    Head,
    Armor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossEvent {
    LoseDefense,
    LaunchRocket,
    RocketImpact,
    SummonHelper,
    Defeated,
}

impl BossState {
    pub fn new(kind: BossKind) -> Self {
        let max_health = match kind {
            BossKind::Hat => HAT_MAX_HEALTH,
            BossKind::Rocket => ROCKET_MAX_HEALTH,
        };
        Self {
            kind,
            cycle: match kind {
                BossKind::Hat => BossCycle::Reposition,
                BossKind::Rocket => BossCycle::PrepareRocket,
            },
            health: max_health,
            max_health,
            cycle_time: 0.0,
            x: 0.50,
            target_x: 0.50,
            phase_two: false,
            helper_spawned: 0,
            helper_timer: 0.0,
            attacks_resolved: 0,
            hits_this_window: 0,
            hit_flash: 0.0,
            interrupted: false,
            rocket: None,
        }
    }

    pub fn sprite_kind(&self) -> EnemyKind {
        match self.kind {
            BossKind::Hat => EnemyKind::HatBoss,
            BossKind::Rocket => EnemyKind::RocketBoss,
        }
    }

    pub fn is_vulnerable(&self) -> bool {
        match self.kind {
            BossKind::Hat => self.cycle == BossCycle::Vulnerable,
            BossKind::Rocket => {
                self.cycle == BossCycle::ExposedCore
                    || (self.cycle == BossCycle::RocketFlying && self.cycle_time >= 0.42)
            }
        }
    }

    pub fn damage(&mut self, hit: BossHit) -> (bool, bool) {
        // Return (registered hit, weak-point/head hit).
        let (damage, weak) = match (self.kind, hit, self.is_vulnerable()) {
            (BossKind::Hat, _, false) => (0, false),
            (BossKind::Hat, BossHit::Head, true) => (3, true),
            (BossKind::Hat, BossHit::WeakPoint, true) => (2, true),
            (BossKind::Hat, BossHit::Armor, true) => (1, false),
            (BossKind::Rocket, BossHit::WeakPoint, true) => (4, true),
            (BossKind::Rocket, BossHit::Head, true) => (2, true),
            (BossKind::Rocket, _, true) => (1, false),
            (BossKind::Rocket, BossHit::Armor, false) => (1, false),
            (BossKind::Rocket, _, false) => return (false, false),
        };
        self.health = (self.health - damage).max(0);
        self.hit_flash = 0.12;
        if self.kind == BossKind::Hat && self.cycle == BossCycle::Vulnerable {
            self.hits_this_window = self.hits_this_window.saturating_add(1);
            if self.hits_this_window >= 3 {
                self.interrupted = true;
            }
        }
        (true, weak)
    }

    pub fn update(&mut self, dt: f32, helper_count: usize) -> Vec<BossEvent> {
        let mut events = Vec::new();
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        self.cycle_time += dt;
        self.helper_timer += dt;
        self.x += (self.target_x - self.x) * (1.0 - (-5.0 * dt).exp());
        if self.health <= 0 && self.cycle != BossCycle::Dying && self.cycle != BossCycle::Dead {
            self.cycle = BossCycle::Dying;
            self.cycle_time = 0.0;
            events.push(BossEvent::Defeated);
            return events;
        }

        if self.kind == BossKind::Hat {
            self.update_hat(&mut events);
        } else {
            self.update_rocket(dt, &mut events);
        }

        if self.kind == BossKind::Rocket && self.health <= self.max_health / 2 {
            self.phase_two = true;
        }
        if self.phase_two && self.helper_spawned < 2 && helper_count == 0 {
            let summon_after = if self.helper_spawned == 0 { 3.5 } else { 4.8 };
            if self.helper_timer >= summon_after && self.cycle != BossCycle::Dying {
                self.helper_spawned += 1;
                self.helper_timer = 0.0;
                events.push(BossEvent::SummonHelper);
            }
        }
        events
    }

    pub fn intercept_rocket(&mut self) -> bool {
        let Some(rocket) = &mut self.rocket else {
            return false;
        };
        if rocket.resolved {
            return false;
        }
        rocket.resolved = true;
        self.cycle = BossCycle::ExposedCore;
        self.cycle_time = 0.0;
        true
    }

    pub fn is_covered(&self) -> bool {
        matches!(self.kind, BossKind::Hat) && self.cycle == BossCycle::Covered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hat_attack_resolves_once_per_attack_window() {
        let mut boss = BossState::new(BossKind::Hat);
        boss.cycle = BossCycle::Attack;
        boss.cycle_time = 0.69;
        let events = boss.update(0.02, 0);
        assert_eq!(
            events
                .iter()
                .filter(|event| **event == BossEvent::LoseDefense)
                .count(),
            1
        );
        assert!(
            boss.update(0.03, 0)
                .iter()
                .all(|event| *event != BossEvent::LoseDefense)
        );
    }

    #[test]
    fn destroyed_rocket_cannot_hit_the_post_twice() {
        let mut boss = BossState::new(BossKind::Rocket);
        boss.cycle = BossCycle::RocketFlying;
        boss.rocket = Some(Rocket {
            x: 0.5,
            y: 0.5,
            progress: 0.3,
            resolved: false,
        });
        assert!(boss.intercept_rocket());
        assert!(!boss.intercept_rocket());
        assert_eq!(boss.cycle, BossCycle::ExposedCore);
    }
}
