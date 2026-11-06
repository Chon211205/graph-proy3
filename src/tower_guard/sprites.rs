use std::{fs::File, io::BufReader};

use crate::framebuffer::Framebuffer;

use super::config::{CharacterDef, EnemyKind, HitBox, load_manifest};

pub struct Sprite {
    width: usize,
    height: usize,
    rgba: Vec<[u8; 4]>,
    opaque: Vec<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct SpritePose {
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub height: f32,
    pub rotation: f32,
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelRegion {
    Head,
    WeakPoint,
    Body,
}

pub struct CharacterSprite {
    pub definition: CharacterDef,
    pub sprite: Sprite,
}

pub struct SpriteBank {
    characters: Vec<CharacterSprite>,
}

impl SpriteBank {
    pub fn load() -> Result<Self, String> {
        let mut characters = Vec::new();
        for definition in load_manifest()? {
            let sprite = Sprite::load(&definition.sprite_path)?;
            characters.push(CharacterSprite { definition, sprite });
        }
        Ok(Self { characters })
    }

    pub fn get(&self, kind: EnemyKind) -> &CharacterSprite {
        self.characters
            .iter()
            .find(|character| character.definition.kind == kind)
            .expect("tower enemy manifest must contain each character exactly once")
    }
}

impl Sprite {
    fn load(path: &str) -> Result<Self, String> {
        let file = File::open(path).map_err(|error| format!("{path}: {error}"))?;
        let mut decoder = png::Decoder::new(BufReader::new(file));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder
            .read_info()
            .map_err(|error| format!("{path}: {error}"))?;
        let mut source = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut source)
            .map_err(|error| format!("{path}: {error}"))?;
        let source = &source[..info.buffer_size()];
        let channels = match info.color_type {
            png::ColorType::Rgba => 4,
            png::ColorType::Rgb => 3,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Grayscale => 1,
            png::ColorType::Indexed => return Err(format!("Unexpanded indexed PNG: {path}")),
        };
        let pixels = source
            .chunks_exact(channels)
            .map(|pixel| match info.color_type {
                png::ColorType::Rgba => [pixel[0], pixel[1], pixel[2], pixel[3]],
                png::ColorType::Rgb => [pixel[0], pixel[1], pixel[2], 255],
                png::ColorType::GrayscaleAlpha => [pixel[0], pixel[0], pixel[0], pixel[1]],
                png::ColorType::Grayscale => [pixel[0], pixel[0], pixel[0], 255],
                png::ColorType::Indexed => unreachable!(),
            })
            .collect::<Vec<_>>();
        let opaque = pixels.iter().map(|pixel| pixel[3] >= 48).collect();
        Ok(Self {
            width: info.width as usize,
            height: info.height as usize,
            rgba: pixels,
            opaque,
        })
    }

    fn source_at(&self, screen_x: f32, screen_y: f32, pose: SpritePose) -> Option<(usize, usize)> {
        if pose.height <= 0.0 || pose.alpha <= 0.0 {
            return None;
        }
        let width = pose.height * self.width as f32 / self.height as f32;
        let (sin, cos) = pose.rotation.sin_cos();
        let dx = screen_x - pose.anchor_x;
        let dy = screen_y - pose.anchor_y;
        // Inverse-rotate the screen pixel around the consistent foot anchor.
        let local_x = cos * dx + sin * dy;
        let local_y = -sin * dx + cos * dy;
        let u = local_x / width + 0.5;
        let v = local_y / pose.height + 1.0;
        if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
            return None;
        }
        Some((
            (u * self.width as f32).floor() as usize,
            (v * self.height as f32).floor() as usize,
        ))
    }

    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn hit_region(
        &self,
        screen_x: f32,
        screen_y: f32,
        pose: SpritePose,
        head: HitBox,
        weak_point: Option<HitBox>,
    ) -> Option<PixelRegion> {
        let (x, y) = self.source_at(screen_x, screen_y, pose)?;
        let index = y * self.width + x;
        if !self.opaque[index] {
            return None;
        }
        let u = x as f32 / self.width as f32;
        let v = y as f32 / self.height as f32;
        if inside(head, u, v) {
            Some(PixelRegion::Head)
        } else if weak_point.is_some_and(|box_| inside(box_, u, v)) {
            Some(PixelRegion::WeakPoint)
        } else {
            Some(PixelRegion::Body)
        }
    }

    pub fn draw(&self, frame: &mut Framebuffer, pose: SpritePose) {
        if pose.height <= 0.0 || pose.alpha <= 0.0 {
            return;
        }
        let width = pose.height * self.width as f32 / self.height as f32;
        let (sin, cos) = pose.rotation.sin_cos();
        let half_width = 0.5 * (width * cos.abs() + pose.height * sin.abs());
        let half_height = 0.5 * (width * sin.abs() + pose.height * cos.abs());
        let left = (pose.anchor_x - half_width - 1.0).floor().max(0.0) as usize;
        let right = (pose.anchor_x + half_width + 1.0)
            .ceil()
            .min(frame.width as f32) as usize;
        let top = (pose.anchor_y - 2.0 * half_height - 1.0).floor().max(0.0) as usize;
        let bottom = (pose.anchor_y + 1.0).ceil().min(frame.height as f32) as usize;
        for y in top..bottom {
            for x in left..right {
                let Some((sx, sy)) = self.source_at(x as f32 + 0.5, y as f32 + 0.5, pose) else {
                    continue;
                };
                let color = self.rgba[sy * self.width + sx];
                let alpha = (color[3] as f32 / 255.0 * pose.alpha).clamp(0.0, 1.0);
                if alpha < 0.01 {
                    continue;
                }
                let index = y * frame.width + x;
                frame.pixels[index] = blend(frame.pixels[index], color, alpha);
            }
        }
    }
}

fn inside(rect: HitBox, x: f32, y: f32) -> bool {
    x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}

fn blend(dst: u32, src: [u8; 4], alpha: f32) -> u32 {
    let channel = |shift: u32, value: u8| {
        let background = ((dst >> shift) & 255) as f32;
        (background + (value as f32 - background) * alpha).round() as u32
    };
    (channel(16, src[0]) << 16) | (channel(8, src[1]) << 8) | channel(0, src[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen_point(sprite: &Sprite, pose: SpritePose, u: f32, v: f32) -> (f32, f32) {
        let width = pose.height * sprite.aspect();
        let local_x = (u - 0.5) * width;
        let local_y = (v - 1.0) * pose.height;
        let (sin, cos) = pose.rotation.sin_cos();
        (
            pose.anchor_x + cos * local_x - sin * local_y,
            pose.anchor_y + sin * local_x + cos * local_y,
        )
    }

    #[test]
    fn alpha_mask_and_head_region_match_the_draw_transform_at_multiple_scales() {
        let bank = SpriteBank::load().expect("prepared tower sprites should load");
        let mut tested = 0;
        for character in &bank.characters {
            let head_pixel = (0..character.sprite.height).find_map(|y| {
                (0..character.sprite.width).find_map(|x| {
                    let u = (x as f32 + 0.5) / character.sprite.width as f32;
                    let v = (y as f32 + 0.5) / character.sprite.height as f32;
                    let margin_x = 0.025;
                    let margin_y = 0.025;
                    let in_head = u >= character.definition.head.left + margin_x
                        && u <= character.definition.head.right - margin_x
                        && v >= character.definition.head.top + margin_y
                        && v <= character.definition.head.bottom - margin_y;
                    (in_head && character.sprite.opaque[y * character.sprite.width + x])
                        .then_some((u, v))
                })
            });
            let Some((u, v)) = head_pixel else {
                panic!(
                    "head hitbox contains no visible pixels for {}",
                    character.definition.name
                );
            };
            tested += 1;
            let transparent_pixel = character
                .sprite
                .opaque
                .iter()
                .position(|opaque| !opaque)
                .map(|index| {
                    (
                        (index % character.sprite.width) as f32 + 0.5,
                        (index / character.sprite.width) as f32 + 0.5,
                    )
                });
            let pose = SpritePose {
                anchor_x: 800.0,
                anchor_y: 760.0,
                height: 480.0,
                rotation: 0.13,
                alpha: 1.0,
            };
            let (x, y) = screen_point(&character.sprite, pose, u, v);
            assert_eq!(
                character.sprite.hit_region(
                    x,
                    y,
                    pose,
                    character.definition.head,
                    character.definition.weak_point,
                ),
                Some(PixelRegion::Head),
                "head mask mismatch for {}",
                character.definition.name
            );

            if let Some((px, py)) = transparent_pixel {
                let u = px / character.sprite.width as f32;
                let v = py / character.sprite.height as f32;
                let (x, y) = screen_point(&character.sprite, pose, u, v);
                assert_eq!(
                    character.sprite.hit_region(
                        x,
                        y,
                        pose,
                        character.definition.head,
                        character.definition.weak_point,
                    ),
                    None,
                    "transparent source pixel should not register for {}",
                    character.definition.name
                );
            }

            if let Some(weak_box) = character.definition.weak_point {
                let weak_pixel = (0..character.sprite.height).find_map(|y| {
                    (0..character.sprite.width).find_map(|x| {
                        let u = (x as f32 + 0.5) / character.sprite.width as f32;
                        let v = (y as f32 + 0.5) / character.sprite.height as f32;
                        let visible = character.sprite.opaque[y * character.sprite.width + x];
                        (visible
                            && u >= weak_box.left + 0.02
                            && u <= weak_box.right - 0.02
                            && v >= weak_box.top + 0.02
                            && v <= weak_box.bottom - 0.02)
                            .then_some((u, v))
                    })
                });
                let (u, v) = weak_pixel.unwrap_or_else(|| {
                    panic!(
                        "weak point contains no visible pixels for {}",
                        character.definition.name
                    )
                });
                let (x, y) = screen_point(&character.sprite, pose, u, v);
                assert_eq!(
                    character.sprite.hit_region(
                        x,
                        y,
                        pose,
                        character.definition.head,
                        Some(weak_box)
                    ),
                    Some(PixelRegion::WeakPoint),
                    "weak-point hitbox mismatch for {}",
                    character.definition.name
                );
            }
        }
        assert_eq!(tested, 8);
    }
}
