use super::{
    boss::{BossHit, BOSS_FIRE_INTERVAL},
    sprites::{PixelRegion, SpriteBank},
    state::{AudioCue, BossCycle, BossState, LifeState, Phase},
    TowerGame,
};

impl TowerGame {
    pub(super) fn fire(&mut self, width: usize, height: usize, sprites: &SpriteBank) {
        if !matches!(self.phase, Phase::Encounter | Phase::Boss)
            || self.fire_cooldown > 0.0
            || self.reload_timer > 0.0
            || (self.phase == Phase::Boss && self.phase_timer > 0.0)
        {
            return;
        }
        let boss_mode = self.phase == Phase::Boss;
        let ammo = if boss_mode {
            &mut self.boss_ammo
        } else {
            &mut self.normal_ammo
        };
        if *ammo == 0 {
            self.set_feedback("SIN MUNICION", 0.45);
            return;
        }
        *ammo -= 1;
        self.fire_cooldown = if boss_mode { BOSS_FIRE_INTERVAL } else { 0.32 };
        self.stats.shots += 1;
        self.queue_audio(AudioCue::Shot);
        self.muzzle_timer = 0.08;
        let shot_x = self.aim.0 * width as f32;
        let shot_y = self.aim.1 * height as f32;

        let mut targets = Vec::<(f32, TargetHit)>::new();
        for (index, enemy) in self.enemies.iter().enumerate() {
            if enemy.resolved || enemy.state != LifeState::Walking {
                continue;
            }
            let pose = self.enemy_pose(enemy, width, height);
            let character = sprites.get(enemy.kind);
            if let Some(region) = character.walking_frame(enemy.elapsed).hit_region(
                shot_x,
                shot_y,
                pose,
                character.definition.head,
                character.definition.weak_point,
            ) {
                targets.push((enemy.depth, TargetHit::Enemy(index, region)));
            }
        }
        if let Some(boss) = &self.boss {
            if !matches!(boss.cycle, BossCycle::Dying | BossCycle::Dead) {
                let boss_depth = 0.74;
                let pose = self.boss_pose(boss, width, height);
                let character = sprites.get(boss.sprite_kind());
                if let Some(region) = character.sprite.hit_region(
                    shot_x,
                    shot_y,
                    pose,
                    character.definition.head,
                    character.definition.weak_point,
                ) {
                    targets.push((boss_depth, TargetHit::Boss(region)));
                }
                if let Some(rocket) = boss.rocket.as_ref().filter(|rocket| !rocket.resolved) {
                    let dx = self.aim.0 - rocket.x;
                    let dy = self.aim.1 - rocket.y;
                    if dx.abs() < 0.024 && dy.abs() < 0.032 && (dx + dy * 0.55).abs() < 0.036 {
                        targets.push((0.98, TargetHit::Rocket));
                    }
                }
            }
        }
        targets.sort_by(|a, b| b.0.total_cmp(&a.0));
        let Some((_, target)) = targets.first().copied() else {
            self.set_feedback("FALLO", 0.45);
            return;
        };
        match target {
            TargetHit::Enemy(index, region) => {
                self.queue_audio(AudioCue::Impact);
                let enemy = &mut self.enemies[index];
                self.stats.hits += 1;
                enemy.hit_flash = 0.12;
                if region == PixelRegion::Head {
                    enemy.killed_by_headshot = true;
                    self.stats.headshots += 1;
                    self.stats.score += 1_000;
                }
                let damage = if region == PixelRegion::Head {
                    enemy.health
                } else {
                    1
                };
                enemy.health = enemy.health.saturating_sub(damage);
                if enemy.health == 0 {
                    enemy.resolved = true;
                    enemy.state = LifeState::Falling;
                    enemy.state_timer = super::ENEMY_FALL_SECONDS;
                    self.stats.kills += 1;
                    self.stats.score += 500;
                    self.hit_marker_headshot = region == PixelRegion::Head;
                    self.set_feedback(
                        if region == PixelRegion::Head {
                            "TIRO A LA CABEZA"
                        } else {
                            "OBJETIVO ELIMINADO"
                        },
                        0.7,
                    );
                } else {
                    self.stats.score += 180;
                    self.hit_marker_headshot = false;
                    self.set_feedback("IMPACTO - AGUANTA", 0.55);
                }
                self.hit_marker_timer = 0.18;
            }
            TargetHit::Boss(region) => {
                let kind = match region {
                    PixelRegion::WeakPoint => BossHit::WeakPoint,
                    PixelRegion::Head => BossHit::Head,
                    PixelRegion::Body => BossHit::Armor,
                };
                let (registered, weak, health) = {
                    let boss = self
                        .boss
                        .as_mut()
                        .expect("boss target requires active boss");
                    let (registered, weak) = boss.damage(kind);
                    (registered, weak, boss.health)
                };
                if registered {
                    self.queue_audio(AudioCue::Impact);
                    self.stats.hits += 1;
                    if weak {
                        self.stats.headshots += u32::from(region == PixelRegion::Head);
                        self.stats.score += if region == PixelRegion::Head {
                            1_400
                        } else {
                            1_200
                        };
                        self.set_feedback("PUNTO DEBIL", 0.6);
                    } else if health > 0 {
                        self.stats.score += 80;
                        self.set_feedback("BLINDAJE - DANIO REDUCIDO", 0.6);
                    }
                    self.hit_marker_headshot = weak;
                    self.hit_marker_timer = 0.18;
                } else {
                    self.set_feedback("ABRIGO CERRADO", 0.5);
                }
            }
            TargetHit::Rocket => {
                if self.boss.as_mut().is_some_and(BossState::intercept_rocket) {
                    self.queue_audio(AudioCue::Impact);
                    self.stats.hits += 1;
                    self.stats.score += 1_800;
                    self.hit_marker_headshot = false;
                    self.hit_marker_timer = 0.18;
                    self.set_feedback("COHETE DESTRUIDO", 0.7);
                }
            }
        }
        self.muzzle_timer = 0.08;
    }
}

#[derive(Clone, Copy)]
enum TargetHit {
    Enemy(usize, PixelRegion),
    Boss(PixelRegion),
    Rocket,
}
