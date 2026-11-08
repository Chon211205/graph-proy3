use raylib::prelude::*;

pub struct TextureData {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<Color>,
}

impl TextureData {
    pub fn from_image(
        image: &mut Image,
    ) -> Self {
        let width =
            image.width();

        let height =
            image.height();

        let mut pixels =
            Vec::with_capacity(
                (width * height)
                    as usize,
            );

        for y in 0..height {
            for x in 0..width {
                pixels.push(
                    image.get_color(
                        x,
                        y,
                    ),
                );
            }
        }

        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn get_pixel(
        &self,
        x: i32,
        y: i32,
    ) -> Color {
        let x =
            x.clamp(
                0,
                self.width - 1,
            );

        let y =
            y.clamp(
                0,
                self.height - 1,
            );

        let indice =
            (
                y * self.width
                    + x
            ) as usize;

        self.pixels[indice]
    }

    pub fn get(
        &self,
        x: i32,
        y: i32,
    ) -> Color {
        let x =
            x.clamp(
                0,
                self.width - 1,
            );

        let y =
            y.clamp(
                0,
                self.height - 1,
            );

        let index =
            (
                y * self.width
                    + x
            ) as usize;

        self.pixels[index]
    }
}