use super::state::{AudioCue, GameAction, InputFrame, Phase};
use crate::{draw_selection_label, framebuffer::Framebuffer};
use std::{f32::consts::PI, fs::File, io::BufReader};

const MAP: &str = include_str!("../../assets/minigames/tower_guardian/mapa_nivel1.txt");
const CELL: f32 = 25.0;
const FOV: f32 = PI / 3.0;

struct Sprite {
    width: usize,
    height: usize,
    pixels: Vec<[u8; 4]>,
}
pub struct SpriteBank {
    zombies: [[Sprite; 3]; 3],
    pistol: [Sprite; 3],
    wall: Sprite,
    door: Sprite,
    floor: Sprite,
    axe: [Sprite; 3],
    flame: [Sprite; 3],
    ammo: Sprite,
    heal: Sprite,
    key: Sprite,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Weapon {
    Pistol,
    Axe,
    Flamethrower,
}
#[derive(Clone, Copy)]
enum Kind {
    Normal,
    Medium,
    Strong,
}
struct Zombie {
    x: f32,
    y: f32,
    health: i32,
    alive: bool,
    kind: Kind,
    animation: f32,
    attack: f32,
}

pub struct TowerGame {
    phase: Phase,
    map: Vec<Vec<char>>,
    x: f32,
    y: f32,
    angle: f32,
    pitch: f32,
    aiming: bool,
    health: i32,
    ammo: i32,
    reserve: i32,
    fuel: i32,
    has_key: bool,
    weapon: Weapon,
    flame_timer: f32,
    fps: u32,
    zombies: Vec<Zombie>,
    elapsed: f32,
    cooldown: f32,
    damage_flash: f32,
    muzzle_flash: f32,
    kills: u32,
    feedback: String,
    feedback_timer: f32,
    audio: [Option<AudioCue>; 4],
}

impl SpriteBank {
    pub fn load() -> Result<Self, String> {
        let p = "assets/minigames/tower_guardian/sprites";
        let trio = |names: [&str; 3]| -> Result<[Sprite; 3], String> {
            Ok([
                load(&format!("{p}/{}", names[0]))?,
                load(&format!("{p}/{}", names[1]))?,
                load(&format!("{p}/{}", names[2]))?,
            ])
        };
        Ok(Self {
            zombies: [
                trio(["zombie1.png", "zombie2.png", "zombie3.png"])?,
                trio(["zombieV21.png", "zombieV22.png", "zombieV23.png"])?,
                trio(["zombieV31.png", "zombieV32.png", "zombieV33.png"])?,
            ],
            pistol: [
                load(&format!("{p}/pistol1.png"))?,
                load(&format!("{p}/pistol2.png"))?,
                load(&format!("{p}/pistol3.png"))?,
            ],
            wall: load(&format!("{p}/wall.png"))?,
            door: load(&format!("{p}/door.png"))?,
            floor: load(&format!("{p}/floor.png"))?,
            axe: [
                load(&format!("{p}/axe1.png"))?,
                load(&format!("{p}/axe2.png"))?,
                load(&format!("{p}/axe3.png"))?,
            ],
            flame: [
                load(&format!("{p}/flamethrow.png"))?,
                load(&format!("{p}/flamethrow2.png"))?,
                load(&format!("{p}/flamethrow3.png"))?,
            ],
            ammo: load(&format!("{p}/ammo.png"))?,
            heal: load(&format!("{p}/heal.png"))?,
            key: load(&format!("{p}/key.png"))?,
        })
    }
}
fn load(path: &str) -> Result<Sprite, String> {
    let file = File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut d = png::Decoder::new(BufReader::new(file));
    d.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = d.read_info().map_err(|e| e.to_string())?;
    let mut data = vec![0; r.output_buffer_size()];
    let i = r.next_frame(&mut data).map_err(|e| e.to_string())?;
    let n = match i.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        _ => return Err(format!("PNG incompatible: {path}")),
    };
    let pixels = data[..i.buffer_size()]
        .chunks_exact(n)
        .map(|v| [v[0], v[1], v[2], if n == 4 { v[3] } else { 255 }])
        .collect();
    Ok(Sprite {
        width: i.width as usize,
        height: i.height as usize,
        pixels,
    })
}

impl TowerGame {
    pub fn new(_: &SpriteBank) -> Self {
        let mut map: Vec<Vec<char>> = MAP
            .lines()
            .map(|l| l.trim_end_matches('\r').chars().collect())
            .collect();
        let width = map.iter().map(Vec::len).max().unwrap_or(1);
        for row in &mut map {
            row.resize(width, '#')
        }
        let mut player = (62.5, 112.5);
        let mut zombies = Vec::new();
        for (y, row) in map.iter_mut().enumerate() {
            for (x, t) in row.iter_mut().enumerate() {
                if *t == 'P' {
                    player = ((x as f32 + 0.5) * CELL, (y as f32 + 0.5) * CELL);
                    *t = ' '
                }
                let kind = match *t {
                    'Z' | 'L' => Some(Kind::Normal),
                    'M' => Some(Kind::Medium),
                    'F' => Some(Kind::Strong),
                    _ => None,
                };
                if let Some(kind) = kind {
                    let health = match kind {
                        Kind::Normal => 100,
                        Kind::Medium => 175,
                        Kind::Strong => 300,
                    };
                    zombies.push(Zombie {
                        x: (x as f32 + 0.5) * CELL,
                        y: (y as f32 + 0.5) * CELL,
                        health,
                        alive: true,
                        kind,
                        animation: x as f32 * 0.13,
                        attack: 0.,
                    });
                    *t = ' '
                }
            }
        }
        Self {
            phase: Phase::Encounter,
            map,
            x: player.0,
            y: player.1,
            angle: 0.,
            pitch: 0.0,
            aiming: false,
            health: 100,
            ammo: 12,
            reserve: 72,
            fuel: 100,
            has_key: false,
            weapon: Weapon::Pistol,
            flame_timer: 0.0,
            fps: 60,
            zombies,
            elapsed: 0.,
            cooldown: 0.,
            damage_flash: 0.,
            muzzle_flash: 0.,
            kills: 0,
            feedback: "MANSION: ELIMINA LAS AMENAZAS".into(),
            feedback_timer: 3.,
            audio: [None; 4],
        }
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn score(&self) -> u32 {
        self.kills * 500
    }
    pub fn pause(&mut self) {
        if self.phase == Phase::Encounter {
            self.phase = Phase::Paused
        }
    }
    pub fn resume(&mut self) {
        if self.phase == Phase::Paused {
            self.phase = Phase::Encounter
        }
    }
    pub fn take_audio_cues(&mut self) -> [Option<AudioCue>; 4] {
        std::mem::replace(&mut self.audio, [None; 4])
    }
    fn cue(&mut self, c: AudioCue) {
        if let Some(s) = self.audio.iter_mut().find(|s| s.is_none()) {
            *s = Some(c)
        }
    }
    pub fn update(
        &mut self,
        dt: f32,
        input: InputFrame,
        width: usize,
        height: usize,
        _: &SpriteBank,
    ) -> GameAction {
        if input.exit {
            return GameAction::Exit;
        }
        if self.phase == Phase::Paused {
            return GameAction::Continue;
        }
        if matches!(self.phase, Phase::Victory | Phase::Defeat) {
            if input.confirm {
                return GameAction::Restart;
            }
            return GameAction::Continue;
        }
        let dt = dt.clamp(0., 0.05);
        self.fps = (1.0 / dt.max(0.001)).round() as u32;
        self.elapsed += dt;
        self.cooldown = (self.cooldown - dt).max(0.);
        self.damage_flash = (self.damage_flash - dt).max(0.);
        self.muzzle_flash = (self.muzzle_flash - dt).max(0.);
        self.feedback_timer = (self.feedback_timer - dt).max(0.);
        self.flame_timer = (self.flame_timer - dt).max(0.0);
        self.weapon = match input.weapon {
            1 => Weapon::Pistol,
            2 => Weapon::Axe,
            3 => Weapon::Flamethrower,
            _ => self.weapon,
        };
        self.aiming = input.aiming;
        let sensitivity = if self.aiming { 2.15 } else { 2.8 };
        self.angle += input.look_x * sensitivity + input.arrow_x as f32 * dt * 1.8;
        self.pitch = (self.pitch - input.look_y * sensitivity).clamp(-0.34, 0.34);
        let f = input.move_y as f32;
        let s = input.move_x as f32;
        self.move_player(
            (self.angle.cos() * f - self.angle.sin() * s) * 72. * dt,
            (self.angle.sin() * f + self.angle.cos() * s) * 72. * dt,
        );
        if input.reload && self.weapon == Weapon::Pistol && self.ammo < 12 && self.reserve > 0 {
            let n = (12 - self.ammo).min(self.reserve);
            self.ammo += n;
            self.reserve -= n;
            self.cue(AudioCue::Reload);
            self.say("Recargando...", 0.8)
        }
        if input.interact {
            self.open_door()
        }
        if input.fire && self.weapon != Weapon::Flamethrower {
            self.attack(width, height)
        }
        if input.fire_held && self.weapon == Weapon::Flamethrower && self.flame_timer <= 0.0 {
            self.flame_timer = 0.10;
            self.attack(width, height)
        }
        self.collect_item();
        self.update_zombies(dt);
        if self.health <= 0 {
            self.phase = Phase::Defeat
        }
        if self.zombies.iter().all(|z| !z.alive) {
            self.phase = Phase::Victory
        }
        GameAction::Continue
    }
    fn say(&mut self, s: &str, t: f32) {
        self.feedback = s.into();
        self.feedback_timer = t
    }
    fn tile(&self, x: i32, y: i32) -> char {
        if x < 0 || y < 0 {
            return '#';
        }
        self.map
            .get(y as usize)
            .and_then(|r| r.get(x as usize))
            .copied()
            .unwrap_or('#')
    }
    fn solid(&self, x: f32, y: f32) -> bool {
        matches!(
            self.tile((x / CELL) as i32, (y / CELL) as i32),
            '#' | 'D' | 'W' | 'X' | 'B' | 'J' | 'G'
        )
    }
    fn move_player(&mut self, dx: f32, dy: f32) {
        if !self.solid(self.x + dx + dx.signum() * 5., self.y) {
            self.x += dx
        }
        if !self.solid(self.x, self.y + dy + dy.signum() * 5.) {
            self.y += dy
        }
    }
    fn open_door(&mut self) {
        let x = ((self.x + self.angle.cos() * 24.) / CELL) as usize;
        let y = ((self.y + self.angle.sin() * 24.) / CELL) as usize;
        if self
            .map
            .get(y)
            .and_then(|r| r.get(x))
            .is_some_and(|c| *c == 'D')
        {
            if !self.has_key {
                self.say("Puerta cerrada - necesitas una llave", 1.2);
                return;
            }
            self.has_key = false;
            self.map[y][x] = ' ';
            self.cue(AudioCue::Door);
            self.say("Puerta abierta", 1.)
        }
    }
    fn attack(&mut self, width: usize, height: usize) {
        if self.weapon == Weapon::Axe {
            self.axe_attack();
            return;
        }
        if self.weapon == Weapon::Flamethrower {
            self.flame_attack();
            return;
        }
        if self.cooldown > 0. {
            return;
        }
        if self.ammo == 0 {
            self.cue(AudioCue::NoAmmo);
            self.say("Sin municion", 1.);
            return;
        }
        self.ammo -= 1;
        self.cooldown = 0.24;
        self.muzzle_flash = 0.08;
        self.cue(AudioCue::Shot);
        let mut best = None;
        for (i, z) in self.zombies.iter().enumerate().filter(|(_, z)| z.alive) {
            let dx = z.x - self.x;
            let dy = z.y - self.y;
            let d = dx.hypot(dy);
            let a = wrap(dy.atan2(dx) - self.angle);
            let sprite_height = height as f32 * 32.0 / d;
            let horizon = height as f32 * (0.5 + self.pitch);
            let center_x = (a / FOV + 0.5) * width as f32;
            let top = horizon + sprite_height * 0.42 - sprite_height;
            let bottom = horizon + sprite_height * 0.42;
            let cross_x = width as f32 * 0.5;
            let cross_y = height as f32 * 0.5;
            let half_width = sprite_height * 0.28;
            if cross_x >= center_x - half_width
                && cross_x <= center_x + half_width
                && cross_y >= top
                && cross_y <= bottom
                && self.ray(self.angle + a) + 3. >= d
                && best.is_none_or(|(_, b, _)| d < b)
            {
                let headshot = cross_y <= top + sprite_height * 0.25;
                best = Some((i, d, headshot))
            }
        }
        if let Some((i, _, headshot)) = best {
            self.zombies[i].health -= if headshot { 100 } else { 50 };
            self.cue(AudioCue::Impact);
            if self.zombies[i].health <= 0 {
                self.zombies[i].alive = false;
                self.kills += 1;
                self.cue(AudioCue::ZombieDie);
                self.say(
                    if headshot {
                        "HEADSHOT"
                    } else {
                        "ENEMIGO ELIMINADO"
                    },
                    0.6,
                )
            } else {
                self.say(if headshot { "HEADSHOT" } else { "IMPACTO" }, 0.5)
            }
        }
    }
    fn axe_attack(&mut self) {
        if self.cooldown > 0.0 {
            return;
        }
        self.cooldown = 0.42;
        self.cue(AudioCue::Axe);
        let mut hit = None;
        for (i, z) in self.zombies.iter().enumerate().filter(|(_, z)| z.alive) {
            let d = (z.x - self.x).hypot(z.y - self.y);
            let a = wrap((z.y - self.y).atan2(z.x - self.x) - self.angle);
            if d <= 45.0
                && a.abs() <= 0.45
                && self.ray(self.angle + a) + 3.0 >= d
                && hit.is_none_or(|(_, best)| d < best)
            {
                hit = Some((i, d))
            }
        }
        if let Some((i, _)) = hit {
            self.zombies[i].health -= 75;
            self.cue(AudioCue::Impact);
            if self.zombies[i].health <= 0 {
                self.zombies[i].alive = false;
                self.kills += 1;
                self.cue(AudioCue::ZombieDie);
                self.say("Enemigo eliminado", 0.6)
            }
        }
    }
    fn flame_attack(&mut self) {
        if self.fuel <= 0 {
            self.cue(AudioCue::NoAmmo);
            self.say("Sin combustible", 0.8);
            return;
        }
        self.fuel -= 1;
        self.cue(AudioCue::Flame);
        let mut killed = 0;
        for z in &mut self.zombies {
            if !z.alive {
                continue;
            }
            let d = (z.x - self.x).hypot(z.y - self.y);
            let a = wrap((z.y - self.y).atan2(z.x - self.x) - self.angle);
            if d <= 125.0 && a.abs() <= 0.38 {
                z.health -= 15;
                if z.health <= 0 {
                    z.alive = false;
                    killed += 1
                }
            }
        }
        if killed > 0 {
            self.kills += killed;
            self.cue(AudioCue::ZombieDie);
            self.say("LLAMAS", 0.5)
        }
    }
    fn collect_item(&mut self) {
        let tx = (self.x / CELL) as usize;
        let ty = (self.y / CELL) as usize;
        let Some(tile) = self.map.get(ty).and_then(|r| r.get(tx)).copied() else {
            return;
        };
        match tile {
            'K' => {
                self.has_key = true;
                self.map[ty][tx] = ' ';
                self.cue(AudioCue::Key);
                self.say("Llave recogida", 1.0)
            }
            'A' => {
                self.reserve += 8;
                self.map[ty][tx] = ' ';
                self.cue(AudioCue::CollectAmmo);
                self.say("Municion recogida", 1.0)
            }
            'H' if self.health < 100 => {
                self.health = (self.health + 25).min(100);
                self.map[ty][tx] = ' ';
                self.cue(AudioCue::Heal);
                self.say("Curacion recogida", 1.0)
            }
            'Q' => {
                self.fuel += 25;
                self.map[ty][tx] = ' ';
                self.cue(AudioCue::CollectAmmo);
                self.say("Combustible recogido", 1.0)
            }
            _ => {}
        }
    }
    fn update_zombies(&mut self, dt: f32) {
        let (px, py) = (self.x, self.y);
        let map = &self.map;
        let mut damage = 0;
        for z in &mut self.zombies {
            if !z.alive {
                continue;
            }
            let dx = px - z.x;
            let dy = py - z.y;
            let d = dx.hypot(dy);
            if d < 340. {
                z.animation += dt;
                z.attack += dt;
                if d > 20. {
                    let speed = match z.kind {
                        Kind::Normal => 22.,
                        Kind::Medium => 27.,
                        Kind::Strong => 24.,
                    };
                    let nx = z.x + dx / d * speed * dt;
                    let ny = z.y + dy / d * speed * dt;
                    if !solid_map(map, nx, z.y) {
                        z.x = nx
                    }
                    if !solid_map(map, z.x, ny) {
                        z.y = ny
                    }
                } else if z.attack > 1. {
                    z.attack = 0.;
                    damage += match z.kind {
                        Kind::Normal => 10,
                        Kind::Medium => 15,
                        Kind::Strong => 25,
                    }
                }
            }
        }
        if damage > 0 {
            if self.weapon == Weapon::Axe && self.aiming {
                damage = (damage / 4).max(1);
            }
            self.health -= damage;
            self.damage_flash = 0.35;
            self.cue(AudioCue::Damage)
        }
    }
    fn ray(&self, a: f32) -> f32 {
        ray_map(&self.map, self.x, self.y, a).0
    }
    pub fn render(&self, frame: &mut Framebuffer, bank: &SpriteBank) {
        let (w, h) = (frame.width, frame.height);
        let horizon = (h as f32 * (0.5 + self.pitch)).clamp(1.0, h as f32 - 1.0) as usize;
        frame.pixels[..horizon * w].fill(0x11131a);
        draw_floor(frame, &bank.floor, self.x, self.y, self.angle, horizon);
        let mut depth = vec![0.; w];
        for (x, dst) in depth.iter_mut().enumerate() {
            let a = self.angle - FOV / 2. + x as f32 / w as f32 * FOV;
            let (raw, tile, side, offset) = ray_map(&self.map, self.x, self.y, a);
            let d = (raw * (a - self.angle).cos()).max(0.1);
            *dst = d;
            let wall = (h as f32 * CELL / d) as i32;
            let top = horizon as i32 - wall / 2;
            let bottom = horizon as i32 + wall / 2;
            let texture = if tile == 'D' { &bank.door } else { &bank.wall };
            let texture_x = (offset * texture.width as f32) as usize % texture.width;
            let light = (1. - d / 850.).clamp(0.18, 1.) * if side { 0.78 } else { 1. };
            for y in top.max(0)..bottom.min(h as i32) {
                let texture_y = (((y - top) as f32 / wall.max(1) as f32) * texture.height as f32)
                    as usize
                    % texture.height;
                let pixel = texture.pixels[texture_y * texture.width + texture_x];
                let base = (pixel[0] as u32) << 16 | (pixel[1] as u32) << 8 | pixel[2] as u32;
                frame.pixels[y as usize * w + x] = shade(base, light)
            }
        }
        for (ty, row) in self.map.iter().enumerate() {
            for (tx, tile) in row.iter().enumerate() {
                let sprite = match tile {
                    'A' => Some(&bank.ammo),
                    'H' => Some(&bank.heal),
                    'K' => Some(&bank.key),
                    'Q' => Some(&bank.ammo),
                    _ => None,
                };
                let Some(sprite) = sprite else { continue };
                let wx = (tx as f32 + 0.5) * CELL;
                let wy = (ty as f32 + 0.5) * CELL;
                let d = (wx - self.x).hypot(wy - self.y);
                let a = wrap((wy - self.y).atan2(wx - self.x) - self.angle);
                if a.abs() > FOV * 0.6 {
                    continue;
                }
                let sx = (a / FOV + 0.5) * w as f32;
                let size = (h as f32 * 13.0 / d).clamp(12.0, h as f32 * 0.35);
                billboard(
                    frame,
                    sprite,
                    sx,
                    horizon as f32 + size * 0.3,
                    size,
                    &depth,
                    d,
                )
            }
        }
        let mut order = self
            .zombies
            .iter()
            .enumerate()
            .filter(|(_, z)| z.alive)
            .map(|(i, z)| (i, (z.x - self.x).hypot(z.y - self.y)))
            .collect::<Vec<_>>();
        order.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (i, d) in order {
            let z = &self.zombies[i];
            let a = wrap((z.y - self.y).atan2(z.x - self.x) - self.angle);
            if a.abs() > FOV * 0.65 {
                continue;
            }
            let sx = (a / FOV + 0.5) * w as f32;
            let size = (h as f32 * 32. / d).clamp(20., h as f32 * 1.3);
            let k = match z.kind {
                Kind::Normal => 0,
                Kind::Medium => 1,
                Kind::Strong => 2,
            };
            billboard(
                frame,
                &bank.zombies[k][((z.animation * 6.) as usize) % 3],
                sx,
                horizon as f32 + size * 0.42,
                size,
                &depth,
                d,
            )
        }
        let weapon_frame = if self.muzzle_flash > 0.04 {
            2
        } else if self.muzzle_flash > 0. {
            1
        } else {
            0
        };
        let held = match self.weapon {
            Weapon::Pistol => &bank.pistol[weapon_frame],
            Weapon::Axe => {
                &bank.axe[if self.cooldown > 0.22 {
                    1
                } else if self.cooldown > 0.0 {
                    2
                } else {
                    0
                }]
            }
            Weapon::Flamethrower => {
                &bank.flame[if self.flame_timer > 0.06 {
                    1
                } else if self.flame_timer > 0.0 {
                    2
                } else {
                    0
                }]
            }
        };
        weapon(
            frame,
            held,
            self.weapon == Weapon::Pistol && self.muzzle_flash > 0.,
        );
        if self.damage_flash > 0. {
            overlay(frame, 0x8f0000, (self.damage_flash / 0.35 * 0.42).min(0.42))
        }
    }
    pub fn draw_hud(&self, f: &mut Framebuffer) {
        let life_color = if self.health > 50 {
            0x55cc66
        } else if self.health > 25 {
            0xe5cf55
        } else {
            0xdd4444
        };
        draw_selection_label(
            f,
            &format!("Vida: {}", self.health.max(0)),
            82,
            f.height as i32 - 34,
            2,
            life_color,
        );
        draw_selection_label(
            f,
            &format!("Municion: {}/{}", self.ammo, self.reserve),
            285,
            f.height as i32 - 34,
            2,
            0xffffff,
        );
        draw_selection_label(
            f,
            &format!("LLAMA: {}", self.fuel),
            285,
            f.height as i32 - 62,
            2,
            0xff9b3d,
        );
        if self.has_key {
            draw_selection_label(f, "Llave", 455, f.height as i32 - 34, 2, 0xffdf55);
        }
        draw_selection_label(
            f,
            &format!("FPS: {}", self.fps.min(999)),
            f.width as i32 - 62,
            18,
            2,
            0x55dd66,
        );
        if self.feedback_timer > 0. {
            draw_selection_label(f, &self.feedback, f.width as i32 / 2, 18, 2, 0xffffff)
        }
        let (cx, cy) = (f.width as i32 / 2, f.height as i32 / 2);
        line(f, cx - 11, cy, cx - 3, cy, 0xffffff);
        line(f, cx + 3, cy, cx + 11, cy, 0xffffff);
        line(f, cx, cy - 11, cx, cy - 3, 0xffffff);
        line(f, cx, cy + 3, cx, cy + 11, 0xffffff);
        if matches!(self.phase, Phase::Victory | Phase::Defeat) {
            overlay(f, 0, 0.66);
            let title = if self.phase == Phase::Victory {
                "NIVEL 1 COMPLETADO"
            } else {
                "HAS MUERTO"
            };
            draw_selection_label(
                f,
                title,
                f.width as i32 / 2,
                f.height as i32 / 2 - 20,
                3,
                if self.phase == Phase::Victory {
                    0x70e090
                } else {
                    0xee5555
                },
            );
            draw_selection_label(
                f,
                "ENTER: REINICIAR   BACKSPACE: SALIR",
                f.width as i32 / 2,
                f.height as i32 / 2 + 35,
                2,
                0xffffff,
            )
        }
    }
}
fn solid_map(m: &[Vec<char>], x: f32, y: f32) -> bool {
    m.get((y / CELL) as usize)
        .and_then(|r| r.get((x / CELL) as usize))
        .is_none_or(|c| matches!(c, '#' | 'D' | 'W' | 'X' | 'B' | 'J' | 'G'))
}
fn ray_map(m: &[Vec<char>], ox: f32, oy: f32, a: f32) -> (f32, char, bool, f32) {
    let (dx, dy) = (a.cos(), a.sin());
    let mut d = 0.;
    let mut prev = (ox / CELL) as i32;
    loop {
        d += 1.5;
        let px = ox + dx * d;
        let py = oy + dy * d;
        let tx = (px / CELL) as i32;
        let ty = (py / CELL) as i32;
        let t = if tx < 0 || ty < 0 {
            '#'
        } else {
            m.get(ty as usize)
                .and_then(|r| r.get(tx as usize))
                .copied()
                .unwrap_or('#')
        };
        if matches!(t, '#' | 'D' | 'W' | 'X' | 'B' | 'J' | 'G') {
            let side = tx != prev;
            let offset = if side {
                (py / CELL).fract()
            } else {
                (px / CELL).fract()
            };
            return (d, t, side, offset);
        }
        prev = tx;
        if d > 1600. {
            return (d, '#', false, 0.0);
        }
    }
}
fn wrap(mut a: f32) -> f32 {
    while a > PI {
        a -= 2. * PI
    }
    while a < -PI {
        a += 2. * PI
    }
    a
}
fn draw_floor(f: &mut Framebuffer, texture: &Sprite, px: f32, py: f32, angle: f32, horizon: usize) {
    let left = angle - FOV * 0.5;
    let right = angle + FOV * 0.5;
    for y in horizon..f.height {
        let row = (y - horizon).max(1) as f32;
        let distance = f.height as f32 * CELL * 0.48 / row;
        let start_x = px + left.cos() * distance;
        let start_y = py + left.sin() * distance;
        let step_x = (right.cos() - left.cos()) * distance / f.width as f32;
        let step_y = (right.sin() - left.sin()) * distance / f.width as f32;
        let fog = (1.0 - distance / 900.0).clamp(0.16, 0.82);
        for x in 0..f.width {
            let world_x = start_x + step_x * x as f32;
            let world_y = start_y + step_y * x as f32;
            let tx = ((world_x / CELL).rem_euclid(1.0) * texture.width as f32) as usize;
            let ty = ((world_y / CELL).rem_euclid(1.0) * texture.height as f32) as usize;
            let p = texture.pixels[ty * texture.width + tx];
            f.pixels[y * f.width + x] =
                shade((p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32, fog);
        }
    }
}
fn shade(c: u32, s: f32) -> u32 {
    let f = |n: u32| (((c >> n) & 255u32) as f32 * s) as u32;
    (f(16) << 16) | (f(8) << 8) | f(0)
}
fn billboard(
    f: &mut Framebuffer,
    s: &Sprite,
    cx: f32,
    ground: f32,
    height: f32,
    depth: &[f32],
    distance: f32,
) {
    let width = height * s.width as f32 / s.height as f32;
    let left = (cx - width / 2.) as i32;
    let top = (ground - height) as i32;
    for y in 0..height as i32 {
        let sy = (y as f32 / height * s.height as f32) as usize;
        if top + y < 0 || top + y >= f.height as i32 {
            continue;
        }
        for x in 0..width as i32 {
            let dx = left + x;
            if dx < 0 || dx >= f.width as i32 || distance > depth[dx as usize] + 5. {
                continue;
            }
            let sx = (x as f32 / width * s.width as f32) as usize;
            let p = s.pixels[sy.min(s.height - 1) * s.width + sx.min(s.width - 1)];
            if p[3] < 40 {
                continue;
            }
            f.pixels[(top + y) as usize * f.width + dx as usize] =
                (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32
        }
    }
}
fn weapon(f: &mut Framebuffer, s: &Sprite, flash: bool) {
    let h = f.height as f32 * 0.39;
    let w = h * s.width as f32 / s.height as f32;
    let left = (f.width as f32 / 2. - w / 2.) as i32;
    let top = (f.height as f32 - h) as i32;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let p = s.pixels[(y as f32 / h * s.height as f32) as usize * s.width
                + (x as f32 / w * s.width as f32) as usize];
            if p[3] > 40 {
                let (dx, dy) = (left + x, top + y);
                if dx >= 0 && dy >= 0 && dx < f.width as i32 && dy < f.height as i32 {
                    f.pixels[dy as usize * f.width + dx as usize] =
                        (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32
                }
            }
        }
    }
    if flash {
        for y in -15..15 {
            for x in -15..15 {
                if x * x + y * y < 220 {
                    let (dx, dy) = (f.width as i32 / 2 + x, top + y);
                    if dx >= 0 && dy >= 0 && dx < f.width as i32 && dy < f.height as i32 {
                        f.pixels[dy as usize * f.width + dx as usize] = 0xffd060
                    }
                }
            }
        }
    }
}
fn overlay(f: &mut Framebuffer, c: u32, a: f32) {
    for p in &mut f.pixels {
        let mix = |n: u32| {
            (((*p >> n) & 255u32) as f32 * (1. - a) + ((c >> n) & 255u32) as f32 * a) as u32
        };
        *p = mix(16) << 16 | mix(8) << 8 | mix(0)
    }
}
fn line(f: &mut Framebuffer, mut x0: i32, mut y0: i32, x1: i32, y1: i32, c: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut e = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 && x0 < f.width as i32 && y0 < f.height as i32 {
            f.pixels[y0 as usize * f.width + x0 as usize] = c
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let q = 2 * e;
        if q >= dy {
            e += dy;
            x0 += sx
        }
        if q <= dx {
            e += dx;
            y0 += sy
        }
    }
}
