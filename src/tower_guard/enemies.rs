use super::{
    config::EnemyKind,
    director::{normal_lifetime, GROUP_WARNING_SECONDS},
    state::{Enemy, LifeState, Phase, Statistics},
    TowerGame,
};

impl TowerGame {
    pub(super) fn make_enemy(&self, kind: EnemyKind, slot: usize, helper: bool) -> Enemy {
        let lanes = [0.28_f32, 0.72, 0.42, 0.58];
        let lane = lanes[(self.group_index + slot + self.round as usize) % lanes.len()];
        let from_left = (self.group_index + slot) % 2 == 0;
        let mut health = self.enemy_health[kind.index()];
        if let Some(kind_health) = helper.then_some(1) {
            health = kind_health;
        }
        Enemy {
            kind,
            state: LifeState::Telegraph,
            health,
            resolved: false,
            start_x: if from_left { 0.06 } else { 0.94 },
            target_x: lane,
            x: if from_left { 0.06 } else { 0.94 },
            depth: 0.18 + slot as f32 * 0.08,
            progress: 0.0,
            lifetime: normal_lifetime(kind),
            elapsed: 0.0,
            state_timer: if helper { 0.28 } else { GROUP_WARNING_SECONDS },
            cover_used: false,
            hit_flash: 0.0,
            killed_by_headshot: false,
            seed: (self.round * 17 + self.group_index as u8 * 5 + slot as u8) as f32 * 0.91,
        }
    }

    pub(super) fn update_enemies(&mut self, dt: f32) {
        for enemy in &mut self.enemies {
            enemy.elapsed += dt;
            enemy.hit_flash = (enemy.hit_flash - dt).max(0.0);
            match enemy.state {
                LifeState::Telegraph => {
                    enemy.state_timer -= dt;
                    if enemy.state_timer <= 0.0 {
                        enemy.state = LifeState::Walking;
                        enemy.elapsed = 0.0;
                    }
                }
                LifeState::Walking => {
                    enemy.progress = (enemy.progress + dt / enemy.lifetime).min(1.0);
                    let smooth = enemy.progress * enemy.progress * (3.0 - 2.0 * enemy.progress);
                    enemy.x = enemy.start_x + (enemy.target_x - enemy.start_x) * smooth;
                    enemy.depth = (0.18 + enemy.progress * 0.51).clamp(0.18, 0.69);
                    // Some fast variants briefly change cover before charging again.
                    if enemy.progress > 0.42
                        && enemy.progress < 0.43 + dt / enemy.lifetime
                        && enemy.kind != EnemyKind::GreenBrute
                        && !enemy.cover_used
                        && self.round >= 2
                    {
                        enemy.state = LifeState::Covered;
                        enemy.state_timer = 0.54;
                        enemy.cover_used = true;
                    } else if enemy.progress >= 1.0 {
                        resolve_escape(enemy, &mut self.stats, &mut self.defenses);
                        enemy.state = LifeState::Escaping;
                        enemy.state_timer = 0.42;
                        self.lost_defense_flash = 0.38;
                    }
                }
                LifeState::Covered => {
                    enemy.state_timer -= dt;
                    if enemy.state_timer <= 0.0 {
                        enemy.state = LifeState::Walking;
                    }
                }
                LifeState::Escaping => {
                    enemy.state_timer -= dt;
                    if enemy.state_timer <= 0.0 {
                        enemy.state = LifeState::Gone;
                    }
                }
                LifeState::Falling => {
                    enemy.state_timer -= dt;
                    if enemy.state_timer <= 0.0 {
                        enemy.state = LifeState::Gone;
                    }
                }
                LifeState::Gone => {}
            }
        }
        if self.defenses == 0 {
            self.phase = Phase::Defeat;
            self.boss = None;
            self.set_feedback("LA TORRE HA CAIDO", 2.0);
        }
    }
}

pub(super) fn resolve_escape(enemy: &mut Enemy, stats: &mut Statistics, defenses: &mut u8) -> bool {
    if enemy.resolved {
        return false;
    }
    enemy.resolved = true;
    if enemy.health > 0 {
        *defenses = defenses.saturating_sub(1);
        stats.escapes += 1;
        true
    } else {
        false
    }
}
