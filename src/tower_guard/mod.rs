mod audio;
mod boss;
mod combat;
mod config;
mod director;
mod enemies;
mod hud;
mod scene;
mod sprites;
mod state;

use self::hud::draw_hud;
use boss::{BOSS_MAGAZINE, BOSS_RELOAD_SECONDS, BossEvent};
use config::EnemyKind;
use director::{
    BETWEEN_GROUP_SECONDS, GROUP_RESOLVE_SECONDS, GROUP_WARNING_SECONDS, NORMAL_GROUP_AMMO, group,
    group_count, round_intro,
};
use enemies::resolve_escape;
use sprites::{SpriteBank, SpritePose};
use state::{AudioCue, BossCycle, BossKind, BossState, Enemy, GameAction, LifeState, Statistics};

pub use audio::TowerAudio;
pub use sprites::SpriteBank as TowerSprites;
pub use state::GameAction as TowerAction;
pub use state::{InputFrame, Phase};

const BOSS_INTRO_SECONDS: f32 = 2.4;
const BOSS_DEFEAT_SECONDS: f32 = 2.1;
const ENEMY_FALL_SECONDS: f32 = 1.1;
const AIM_ARROW_SPEED: f32 = 0.62;
const MAX_HELPERS: usize = 2;
const MAX_NORMAL_ENEMIES: usize = 2;

pub struct TowerGame {
    phase: Phase,
    resume_phase: Option<Phase>,
    phase_timer: f32,
    round: u8,
    group_index: usize,
    group_elapsed: f32,
    enemies: Vec<Enemy>,
    boss: Option<BossState>,
    defenses: u8,
    normal_ammo: u8,
    boss_ammo: u8,
    reload_timer: f32,
    fire_cooldown: f32,
    aim: (f32, f32),
    stats: Statistics,
    elapsed: f32,
    feedback: String,
    feedback_timer: f32,
    muzzle_timer: f32,
    hit_marker_timer: f32,
    hit_marker_headshot: bool,
    lost_defense_flash: f32,
    enemy_health: [u8; 8],
    audio_cues: [Option<AudioCue>; 4],
}

impl TowerGame {
    pub fn new(sprites: &SpriteBank) -> Self {
        let enemy_health =
            std::array::from_fn(|index| sprites.get(EnemyKind::ALL[index]).definition.hit_points);
        Self {
            phase: Phase::Intro,
            resume_phase: None,
            phase_timer: round_intro(1),
            round: 1,
            group_index: 0,
            group_elapsed: 0.0,
            enemies: Vec::with_capacity(MAX_NORMAL_ENEMIES + 2),
            boss: None,
            defenses: 5,
            normal_ammo: NORMAL_GROUP_AMMO,
            boss_ammo: BOSS_MAGAZINE,
            reload_timer: 0.0,
            fire_cooldown: 0.0,
            aim: (0.5, 0.52),
            stats: Statistics::default(),
            elapsed: 0.0,
            feedback: "APUNTA Y DISPARA".to_string(),
            feedback_timer: 1.7,
            muzzle_timer: 0.0,
            hit_marker_timer: 0.0,
            hit_marker_headshot: false,
            lost_defense_flash: 0.0,
            enemy_health,
            audio_cues: [None; 4],
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn score(&self) -> u32 {
        self.stats.score
    }

    pub fn take_audio_cues(&mut self) -> [Option<AudioCue>; 4] {
        std::mem::replace(&mut self.audio_cues, [None; 4])
    }

    fn queue_audio(&mut self, cue: AudioCue) {
        if let Some(slot) = self.audio_cues.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(cue);
        }
    }

    pub fn pause(&mut self) {
        if self.phase != Phase::Paused && !matches!(self.phase, Phase::Victory | Phase::Defeat) {
            self.resume_phase = Some(self.phase);
            self.phase = Phase::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = self.resume_phase.take().unwrap_or(Phase::Encounter);
        }
    }

    pub fn update(
        &mut self,
        delta_time: f32,
        input: InputFrame,
        width: usize,
        height: usize,
        sprites: &SpriteBank,
    ) -> GameAction {
        if self.phase == Phase::Paused {
            return GameAction::Continue;
        }
        let dt = delta_time.clamp(0.0, 0.05);
        self.elapsed += dt;
        self.fire_cooldown = (self.fire_cooldown - dt).max(0.0);
        self.feedback_timer = (self.feedback_timer - dt).max(0.0);
        self.muzzle_timer = (self.muzzle_timer - dt).max(0.0);
        self.hit_marker_timer = (self.hit_marker_timer - dt).max(0.0);
        self.lost_defense_flash = (self.lost_defense_flash - dt).max(0.0);
        self.aim_input(input, dt);

        if matches!(self.phase, Phase::Victory | Phase::Defeat) {
            if input.confirm {
                return GameAction::Restart;
            }
            if input.exit {
                return GameAction::Exit;
            }
            if input.mouse_clicked && button_contains(self.aim, 0.37, 0.69, 0.63, 0.77) {
                return GameAction::Restart;
            }
            if input.mouse_clicked && button_contains(self.aim, 0.37, 0.79, 0.63, 0.87) {
                return GameAction::Exit;
            }
            return GameAction::Continue;
        }

        if input.reload
            && self.phase == Phase::Boss
            && self.boss_ammo < BOSS_MAGAZINE
            && self.reload_timer <= 0.0
        {
            self.reload_timer = BOSS_RELOAD_SECONDS;
            self.set_feedback("RECARGANDO", 1.1);
        }
        if self.reload_timer > 0.0 {
            self.reload_timer = (self.reload_timer - dt).max(0.0);
            if self.reload_timer == 0.0 {
                self.boss_ammo = BOSS_MAGAZINE;
                self.set_feedback("CARGADOR LISTO", 0.8);
            }
        }

        match self.phase {
            Phase::Intro => self.update_intro(dt),
            Phase::Encounter => self.update_encounter(dt),
            Phase::Resolving => self.update_resolution(dt),
            Phase::Transition => self.update_transition(dt),
            Phase::Boss => self.update_boss(dt),
            Phase::Paused | Phase::Victory | Phase::Defeat => {}
        }

        if input.fire {
            self.fire(width, height, sprites);
        }
        GameAction::Continue
    }

    fn aim_input(&mut self, input: InputFrame, dt: f32) {
        if input.cursor_moved {
            self.aim = input.cursor;
        }
        if input.arrow_x != 0 || input.arrow_y != 0 {
            self.aim.0 += input.arrow_x as f32 * AIM_ARROW_SPEED * dt;
            self.aim.1 += input.arrow_y as f32 * AIM_ARROW_SPEED * dt;
        }
        self.aim.0 = self.aim.0.clamp(0.015, 0.985);
        self.aim.1 = self.aim.1.clamp(0.08, 0.97);
    }

    fn update_intro(&mut self, dt: f32) {
        self.phase_timer -= dt;
        if self.phase_timer > 0.0 {
            return;
        }
        if self.round == 3 || self.round == 5 {
            let kind = if self.round == 3 {
                BossKind::Hat
            } else {
                BossKind::Rocket
            };
            self.start_boss(kind);
        } else {
            self.begin_group();
        }
    }

    fn begin_group(&mut self) {
        self.enemies.clear();
        self.group_elapsed = 0.0;
        self.normal_ammo = NORMAL_GROUP_AMMO;
        let Some(kinds) = group(self.round, self.group_index) else {
            self.start_transition();
            return;
        };
        for (slot, kind) in kinds.iter().copied().take(MAX_NORMAL_ENEMIES).enumerate() {
            self.enemies.push(self.make_enemy(kind, slot, false));
        }
        self.phase = Phase::Encounter;
        self.set_feedback("OBJETIVOS ENTRANDO", GROUP_WARNING_SECONDS);
    }

    fn start_transition(&mut self) {
        self.phase = Phase::Transition;
        self.phase_timer = BETWEEN_GROUP_SECONDS;
    }

    fn update_encounter(&mut self, dt: f32) {
        self.group_elapsed += dt;
        self.update_enemies(dt);
        if self.phase == Phase::Defeat {
            return;
        }
        let pending = self.enemies.iter().any(|enemy| !enemy.resolved);
        if !pending {
            self.phase = Phase::Resolving;
            self.phase_timer = GROUP_RESOLVE_SECONDS;
            self.set_feedback("GRUPO ELIMINADO", GROUP_RESOLVE_SECONDS);
        } else if self.normal_ammo == 0 {
            self.phase = Phase::Resolving;
            self.phase_timer = GROUP_RESOLVE_SECONDS;
            self.set_feedback("OBJETIVOS ESCAPANDO", GROUP_RESOLVE_SECONDS);
        } else if self.group_elapsed >= 9.1 {
            self.phase = Phase::Resolving;
            self.phase_timer = GROUP_RESOLVE_SECONDS;
            self.set_feedback("SE AGOTO LA VENTANA", GROUP_RESOLVE_SECONDS);
        }
    }

    fn update_resolution(&mut self, dt: f32) {
        self.update_enemies(dt);
        self.phase_timer -= dt;
        if self.phase_timer > 0.0 {
            return;
        }
        for enemy in &mut self.enemies {
            resolve_escape(enemy, &mut self.stats, &mut self.defenses);
            if enemy.state != LifeState::Gone {
                enemy.state = LifeState::Gone;
            }
        }
        if self.defenses == 0 {
            self.phase = Phase::Defeat;
            self.set_feedback("LA TORRE HA CAIDO", 2.0);
            return;
        }
        self.phase = Phase::Transition;
        self.phase_timer = BETWEEN_GROUP_SECONDS;
    }

    fn update_transition(&mut self, dt: f32) {
        self.phase_timer -= dt;
        if self.phase_timer > 0.0 {
            return;
        }
        if self.round == 3 || self.round == 5 {
            if self.round == 5 {
                self.phase = Phase::Victory;
                self.set_feedback("ULTIMA GUARDIA SUPERADA", 3.0);
            } else {
                self.round = 4;
                self.group_index = 0;
                self.phase = Phase::Intro;
                self.phase_timer = round_intro(self.round);
            }
        } else if self.group_index + 1 < group_count(self.round) {
            self.group_index += 1;
            self.begin_group();
        } else {
            self.round += 1;
            self.group_index = 0;
            if self.round > 5 {
                self.phase = Phase::Victory;
                self.set_feedback("ULTIMA GUARDIA SUPERADA", 3.0);
            } else {
                self.phase = Phase::Intro;
                self.phase_timer = round_intro(self.round);
                self.set_feedback("PREPARATE", self.phase_timer);
            }
        }
    }

    fn start_boss(&mut self, kind: BossKind) {
        self.queue_audio(AudioCue::BossAlert);
        self.enemies.clear();
        self.boss = Some(BossState::new(kind));
        self.boss_ammo = BOSS_MAGAZINE;
        self.reload_timer = 0.0;
        self.phase = Phase::Boss;
        self.phase_timer = BOSS_INTRO_SECONDS;
        self.set_feedback(
            if kind == BossKind::Hat {
                "JEFE DEL SOMBRERO"
            } else {
                "JEFE LANZACOHETES"
            },
            BOSS_INTRO_SECONDS,
        );
    }

    fn update_boss(&mut self, dt: f32) {
        if self.phase_timer > 0.0 {
            self.phase_timer -= dt;
            return;
        }
        self.update_enemies(dt);
        if self.phase == Phase::Defeat {
            return;
        }
        let helpers = self.enemies.iter().filter(|enemy| !enemy.resolved).count();
        let (events, helper_spawned) = {
            let Some(boss) = &mut self.boss else {
                return;
            };
            let events = boss.update(dt, helpers);
            (events, boss.helper_spawned)
        };
        for event in events {
            match event {
                BossEvent::LoseDefense | BossEvent::RocketImpact => {
                    self.lose_defense();
                }
                BossEvent::LaunchRocket => {
                    self.queue_audio(AudioCue::BossAlert);
                    self.set_feedback("COHETE EN CAMINO", 0.75);
                }
                BossEvent::SummonHelper => {
                    self.queue_audio(AudioCue::BossAlert);
                    if self.enemies.len() < MAX_NORMAL_ENEMIES + MAX_HELPERS {
                        let helper_kind = if helper_spawned == 1 {
                            EnemyKind::BlackCreature
                        } else {
                            EnemyKind::OrangeClaw
                        };
                        let helper = self.make_enemy(helper_kind, helper_spawned as usize, true);
                        self.enemies.push(helper);
                        self.set_feedback("CONTACTO EN LOS FLANCOS", 1.1);
                    }
                }
                BossEvent::Defeated => {
                    self.queue_audio(AudioCue::Impact);
                    self.set_feedback("JEFE ELIMINADO", BOSS_DEFEAT_SECONDS);
                }
            }
        }
        if self
            .boss
            .as_ref()
            .is_some_and(|boss| boss.cycle == BossCycle::Dead)
        {
            self.boss = None;
            self.phase = Phase::Resolving;
            self.phase_timer = BOSS_DEFEAT_SECONDS;
            self.enemies.clear();
            self.group_index = group_count(self.round).saturating_sub(1);
        }
    }

    fn lose_defense(&mut self) {
        self.queue_audio(AudioCue::Damage);
        self.defenses = self.defenses.saturating_sub(1);
        self.stats.escapes += 1;
        self.lost_defense_flash = 0.42;
        if self.defenses == 0 {
            self.phase = Phase::Defeat;
            self.boss = None;
            self.set_feedback("LA TORRE HA CAIDO", 2.0);
        } else {
            self.set_feedback("DEFENSA IMPACTADA", 1.0);
        }
    }

    fn set_feedback(&mut self, message: &str, duration: f32) {
        self.feedback.clear();
        self.feedback.push_str(message);
        self.feedback_timer = duration;
    }

    pub fn render(&self, frame: &mut crate::framebuffer::Framebuffer, sprites: &SpriteBank) {
        scene::draw_arena(frame, self.elapsed);
        let mut ordered = self.enemies.iter().collect::<Vec<_>>();
        ordered.sort_by(|a, b| a.depth.total_cmp(&b.depth));
        for enemy in ordered {
            self.draw_enemy(frame, sprites, enemy);
        }
        if let Some(boss) = &self.boss {
            self.draw_boss(frame, sprites, boss);
        }
        scene::draw_foreground_cover(frame);
        if let Some(boss) = &self.boss {
            if let Some(rocket) = &boss.rocket {
                if !rocket.resolved {
                    draw_rocket(frame, rocket.x, rocket.y, rocket.progress, self.elapsed);
                }
            }
        }
    }

    fn draw_enemy(
        &self,
        frame: &mut crate::framebuffer::Framebuffer,
        sprites: &SpriteBank,
        enemy: &Enemy,
    ) {
        if enemy.state == LifeState::Gone {
            return;
        }
        let pose = self.enemy_pose(enemy, frame.width, frame.height);
        draw_shadow(frame, pose.anchor_x, pose.anchor_y, pose.height * 0.24);
        if enemy.state == LifeState::Telegraph {
            draw_warning(frame, enemy.x, 0.58 + enemy.depth * 0.32, enemy.elapsed);
            return;
        }
        sprites.get(enemy.kind).sprite.draw(frame, pose);
        if enemy.state != LifeState::Falling {
            draw_enemy_health(frame, pose, enemy.health);
        }
        if enemy.hit_flash > 0.0 {
            let (r, g, b) = if enemy.killed_by_headshot {
                (255, 204, 92)
            } else {
                (255, 238, 218)
            };
            draw_hit_spark(
                frame,
                pose.anchor_x,
                pose.anchor_y - pose.height * 0.63,
                [r, g, b],
                enemy.hit_flash,
            );
        }
        if enemy.state == LifeState::Falling {
            draw_fall_dust(
                frame,
                pose.anchor_x,
                pose.anchor_y,
                ENEMY_FALL_SECONDS - enemy.state_timer,
            );
        }
    }

    fn draw_boss(
        &self,
        frame: &mut crate::framebuffer::Framebuffer,
        sprites: &SpriteBank,
        boss: &BossState,
    ) {
        if boss.cycle == BossCycle::Dead {
            return;
        }
        let mut pose = self.boss_pose(boss, frame.width, frame.height);
        if boss.cycle == BossCycle::Dying {
            let fall = (boss.cycle_time / BOSS_DEFEAT_SECONDS).clamp(0.0, 1.0);
            pose.rotation = fall * 1.2;
            pose.alpha = 1.0 - fall;
            pose.anchor_y += fall * pose.height * 0.16;
        } else if boss.cycle == BossCycle::Windup {
            pose.rotation = (boss.cycle_time * 5.0).sin() * 0.025;
        }
        draw_shadow(frame, pose.anchor_x, pose.anchor_y, pose.height * 0.31);
        sprites.get(boss.sprite_kind()).sprite.draw(frame, pose);
        if boss.is_covered() {
            draw_cover_alert(frame, boss.x, pose.anchor_y / frame.height as f32);
        }
        if boss.cycle == BossCycle::Windup || boss.cycle == BossCycle::PrepareRocket {
            draw_warning(
                frame,
                boss.x,
                pose.anchor_y / frame.height as f32 - 0.06,
                self.elapsed,
            );
        }
        if boss.is_vulnerable() {
            let flash = if self.elapsed.rem_euclid(0.42) < 0.22 {
                0xffb642
            } else {
                0xffe4a1
            };
            if let Some(weak) = sprites.get(boss.sprite_kind()).definition.weak_point {
                let x = pose.anchor_x
                    + (weak.left + weak.right - 1.0)
                        * pose.height
                        * 0.5
                        * sprites.get(boss.sprite_kind()).sprite.aspect();
                let y = pose.anchor_y - pose.height + (weak.top + weak.bottom) * pose.height * 0.5;
                draw_weak_glow(frame, x, y, pose.height * 0.07, flash);
            }
        }
        if boss.hit_flash > 0.0 {
            draw_hit_spark(
                frame,
                pose.anchor_x,
                pose.anchor_y - pose.height * 0.68,
                [255, 225, 188],
                boss.hit_flash,
            );
        }
    }

    fn enemy_pose(&self, enemy: &Enemy, width: usize, height: usize) -> SpritePose {
        let depth_scale = 0.72 + enemy.depth * 0.62;
        let sprite_height = height as f32 * base_height(enemy.kind) * depth_scale;
        let mut x =
            enemy.x + (enemy.elapsed * 5.1 + enemy.seed).sin() * (0.009 + enemy.depth * 0.007);
        let mut y = 0.57 + enemy.depth * 0.33;
        let mut rotation = (enemy.elapsed * 7.4 + enemy.seed).sin() * 0.035;
        let mut alpha = 1.0;
        let h = sprite_height;
        match enemy.state {
            LifeState::Walking => {
                y += (enemy.elapsed * 12.5 + enemy.seed).sin() * 0.006;
            }
            LifeState::Covered => {
                alpha = 0.84;
            }
            LifeState::Escaping => {
                x += (1.0 - enemy.state_timer / 0.42)
                    * if enemy.target_x < 0.5 { -0.30 } else { 0.30 };
                alpha = (enemy.state_timer / 0.42).clamp(0.0, 1.0);
            }
            LifeState::Falling => {
                let fall = (1.0 - enemy.state_timer / ENEMY_FALL_SECONDS).clamp(0.0, 1.0);
                rotation += fall * 1.25;
                y += fall * 0.085;
                alpha = 1.0 - ((fall - 0.72) / 0.28).clamp(0.0, 1.0);
            }
            LifeState::Telegraph | LifeState::Gone => {}
        }
        SpritePose {
            anchor_x: x * width as f32,
            anchor_y: y * height as f32,
            height: h,
            rotation,
            alpha,
        }
    }

    fn boss_pose(&self, boss: &BossState, width: usize, height: usize) -> SpritePose {
        let size = if boss.kind == BossKind::Hat {
            0.51
        } else {
            0.53
        };
        let walking = boss.cycle == BossCycle::Reposition || boss.cycle == BossCycle::Recover;
        let gait = if walking {
            (boss.cycle_time * 8.0).sin()
        } else {
            0.0
        };
        SpritePose {
            anchor_x: boss.x * width as f32,
            anchor_y: (0.83 + gait.abs() * 0.008) * height as f32,
            height: size * (1.0 + gait.abs() * 0.012) * height as f32,
            rotation: gait * 0.014,
            alpha: 1.0,
        }
    }

    pub fn draw_hud(&self, frame: &mut crate::framebuffer::Framebuffer) {
        draw_hud(self, frame);
    }
}

fn base_height(kind: EnemyKind) -> f32 {
    match kind {
        EnemyKind::GreenBrute | EnemyKind::OrangeClaw | EnemyKind::BlueClaw => 0.41,
        EnemyKind::HatBoss => 0.56,
        EnemyKind::RocketBoss => 0.59,
        _ => 0.36,
    }
}

fn draw_enemy_health(frame: &mut crate::framebuffer::Framebuffer, pose: SpritePose, health: u8) {
    if health <= 1 {
        return;
    }
    let scale = (pose.height / 150.0).clamp(0.65, 1.25);
    let pip_width = (8.0 * scale).round() as i32;
    let pip_height = (4.0 * scale).round() as i32;
    let gap = (3.0 * scale).round() as i32;
    let total = i32::from(health) * pip_width + (i32::from(health) - 1) * gap;
    let left = pose.anchor_x.round() as i32 - total / 2;
    let top = (pose.anchor_y - pose.height - 8.0 * scale).round() as i32;
    for pip in 0..i32::from(health) {
        let x = left + pip * (pip_width + gap);
        for y in 0..pip_height {
            for dx in 0..pip_width {
                put(
                    frame,
                    x + dx,
                    top + y,
                    if y == 0 { 0xffd078 } else { 0xd65336 },
                );
            }
        }
    }
}

fn button_contains(point: (f32, f32), left: f32, top: f32, right: f32, bottom: f32) -> bool {
    point.0 >= left && point.0 <= right && point.1 >= top && point.1 <= bottom
}

fn draw_shadow(frame: &mut crate::framebuffer::Framebuffer, x: f32, y: f32, width: f32) {
    let center_x = x as i32;
    let center_y = y as i32 - 2;
    let radius_x = (width * 0.5) as i32;
    let radius_y = (width * 0.075).max(2.0) as i32;
    for dy in -radius_y..=radius_y {
        for dx in -radius_x..=radius_x {
            let norm = dx * dx * radius_y * radius_y + dy * dy * radius_x * radius_x;
            let limit = radius_x * radius_x * radius_y * radius_y;
            if norm <= limit {
                let px = center_x + dx;
                let py = center_y + dy;
                if px >= 0 && py >= 0 && px < frame.width as i32 && py < frame.height as i32 {
                    let at = py as usize * frame.width + px as usize;
                    let old = frame.pixels[at];
                    frame.pixels[at] = ((old & 0xfefefe) >> 1) & 0x1e1e1e;
                }
            }
        }
    }
}

fn draw_warning(frame: &mut crate::framebuffer::Framebuffer, x: f32, y: f32, time: f32) {
    let pulse = 1.0 + (time * 9.0).sin().abs() * 0.34;
    let cx = (x * frame.width as f32) as i32;
    let cy = (y * frame.height as f32) as i32;
    let r = (9.0 * pulse) as i32;
    for angle in 0..12 {
        let a = angle as f32 * std::f32::consts::TAU / 12.0;
        let px = cx + (a.cos() * r as f32) as i32;
        let py = cy + (a.sin() * r as f32) as i32;
        put(frame, px, py, 0xffba55);
    }
    line_px(frame, cx - 3, cy + 4, cx + 3, cy + 4, 0xffba55);
    line_px(frame, cx, cy - 4, cx, cy + 1, 0xffba55);
}

fn draw_cover_alert(frame: &mut crate::framebuffer::Framebuffer, x: f32, y: f32) {
    let px = (x * frame.width as f32) as i32;
    let py = (y * frame.height as f32) as i32 - 12;
    line_px(frame, px - 15, py, px + 15, py, 0x9b6540);
    line_px(frame, px - 15, py + 3, px + 15, py + 3, 0x9b6540);
}

fn draw_hit_spark(
    frame: &mut crate::framebuffer::Framebuffer,
    x: f32,
    y: f32,
    rgb: [u8; 3],
    timer: f32,
) {
    let radius = (1.0 - timer / 0.14).clamp(0.0, 1.0) * 9.0 + 3.0;
    let color = ((rgb[0] as u32) << 16) | ((rgb[1] as u32) << 8) | rgb[2] as u32;
    let px = x as i32;
    let py = y as i32;
    let r = radius as i32;
    line_px(frame, px - r, py, px + r, py, color);
    line_px(frame, px, py - r, px, py + r, color);
}

fn draw_fall_dust(frame: &mut crate::framebuffer::Framebuffer, x: f32, y: f32, timer: f32) {
    if timer > 0.45 {
        return;
    }
    let fade = (timer / 0.45).clamp(0.0, 1.0);
    let radius = ((1.0 - fade) * 18.0) as i32;
    for i in 0..5 {
        let shift = (i as f32 - 2.0) * radius as f32 * 0.45;
        put(frame, x as i32 + shift as i32, y as i32, 0x776350);
    }
}

fn draw_weak_glow(
    frame: &mut crate::framebuffer::Framebuffer,
    x: f32,
    y: f32,
    radius: f32,
    color: u32,
) {
    let r = radius.max(4.0) as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            let dist = (dx * dx + dy * dy) as f32;
            if dist < (r * r) as f32 && ((dx + dy) & 1) == 0 {
                let amount = 1.0 - dist / (r * r) as f32;
                let c = scale_rgb(color, amount * 0.6);
                put(frame, x as i32 + dx, y as i32 + dy, c);
            }
        }
    }
}

fn draw_rocket(
    frame: &mut crate::framebuffer::Framebuffer,
    x: f32,
    y: f32,
    progress: f32,
    time: f32,
) {
    let px = (x * frame.width as f32) as i32;
    let py = (y * frame.height as f32) as i32;
    let radius = (12.0 + progress * 15.0) as i32;
    let wobble = (time * 13.0).sin() as i32;
    for offset in -radius..=radius {
        put(frame, px + offset, py + offset / 2 + wobble, 0xe0d4b3);
        put(frame, px + offset, py + offset / 2 + wobble + 1, 0x7e3325);
    }
    for flame in 1..5 {
        let back = px - radius - flame * 5;
        let color = if flame % 2 == 0 { 0xffb53e } else { 0xd63f27 };
        line_px(frame, back, py - flame, px - radius, py + flame, color);
    }
}

fn put(frame: &mut crate::framebuffer::Framebuffer, x: i32, y: i32, color: u32) {
    if x >= 0 && y >= 0 && x < frame.width as i32 && y < frame.height as i32 {
        frame.pixels[y as usize * frame.width + x as usize] = color;
    }
}

fn line_px(
    frame: &mut crate::framebuffer::Framebuffer,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        put(
            frame,
            (x0 as f32 + (x1 - x0) as f32 * t) as i32,
            (y0 as f32 + (y1 - y0) as f32 * t) as i32,
            color,
        );
    }
}

fn scale_rgb(color: u32, scale: f32) -> u32 {
    let scale = scale.clamp(0.0, 1.0);
    let r = (((color >> 16) & 255) as f32 * scale) as u32;
    let g = (((color >> 8) & 255) as f32 * scale) as u32;
    let b = ((color & 255) as f32 * scale) as u32;
    (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_enemy_escape_reduces_defense_only_once() {
        let mut enemy = Enemy {
            kind: EnemyKind::WhiteShirt,
            state: LifeState::Escaping,
            health: 1,
            resolved: false,
            start_x: 0.0,
            target_x: 1.0,
            x: 1.0,
            depth: 0.5,
            progress: 1.0,
            lifetime: 1.0,
            elapsed: 0.0,
            state_timer: 0.0,
            cover_used: false,
            hit_flash: 0.0,
            killed_by_headshot: false,
            seed: 0.0,
        };
        let mut stats = Statistics::default();
        let mut defenses = 5;

        assert!(resolve_escape(&mut enemy, &mut stats, &mut defenses));
        assert!(!resolve_escape(&mut enemy, &mut stats, &mut defenses));
        assert_eq!(defenses, 4);
        assert_eq!(stats.escapes, 1);
    }

    #[test]
    fn covered_enemy_rejoins_its_approach_path_instead_of_hiding_forever() {
        let sprites = SpriteBank::load().expect("prepared tower sprites should load");
        let mut game = TowerGame::new(&sprites);
        game.round = 2;
        let mut enemy = game.make_enemy(EnemyKind::OrangeClaw, 0, false);
        enemy.state = LifeState::Walking;
        enemy.progress = 0.419;
        enemy.lifetime = 1.0;
        game.enemies.push(enemy);

        game.update_enemies(0.01);
        assert_eq!(game.enemies[0].state, LifeState::Covered);
        assert!(game.enemies[0].cover_used);
        game.enemies[0].state_timer = 0.0;
        game.update_enemies(0.01);
        game.update_enemies(0.01);
        assert_eq!(game.enemies[0].state, LifeState::Walking);
        assert!(game.enemies[0].progress > 0.42);
    }

    #[test]
    fn pause_freezes_the_gameplay_clock() {
        let sprites = SpriteBank::load().expect("prepared tower sprites should load");
        let mut game = TowerGame::new(&sprites);
        game.pause();
        let before = game.elapsed;
        game.update(2.0, InputFrame::default(), 1200, 800, &sprites);
        assert_eq!(game.elapsed, before);
        assert_eq!(game.phase(), Phase::Paused);
    }

    #[test]
    fn write_gameplay_preview_when_capture_path_is_set() {
        let Ok(path) = std::env::var("TOWER_GUARD_CAPTURE") else {
            return;
        };
        let sprites = SpriteBank::load().expect("prepared tower sprites should load");
        let mut game = TowerGame::new(&sprites);
        game.round = 2;
        game.begin_group();
        for (index, enemy) in game.enemies.iter_mut().enumerate() {
            enemy.state = LifeState::Walking;
            enemy.progress = 0.38 + index as f32 * 0.12;
            enemy.depth = 0.36 + index as f32 * 0.12;
            enemy.x = 0.39 + index as f32 * 0.23;
            enemy.elapsed = 1.2 + index as f32 * 0.5;
        }
        let mut frame = crate::framebuffer::Framebuffer::new(1200, 800);
        game.render(&mut frame, &sprites);
        game.draw_hud(&mut frame);
        frame
            .save_png(&path)
            .expect("preview screenshot should be writable");
    }
}
