
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    depth: Vec<f32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
            depth: vec![f32::NEG_INFINITY; width * height],
        }
    }

    pub fn clear(&mut self, background: u32) {
        self.pixels.fill(background);
        self.depth.fill(f32::NEG_INFINITY);
    }

    pub fn pixel(
        &mut self,
        x: i32,
        y: i32,
        z: f32,
        color: u32,
    ) {
        if x < 0
            || y < 0
            || x >= self.width as i32
            || y >= self.height as i32
        {
            return;
        }

        let index = y as usize * self.width + x as usize;

        if z > self.depth[index] {
            self.depth[index] = z;
            self.pixels[index] = color;
        }
    }

    pub fn save_png(
        &self,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 3);

        for pixel in &self.pixels {
            bytes.extend_from_slice(&[
                ((pixel >> 16) & 255) as u8,
                ((pixel >> 8) & 255) as u8,
                (pixel & 255) as u8,
            ]);
        }

        let file = std::fs::File::create(path)?;

        let mut encoder = png::Encoder::new(
            file,
            self.width as u32,
            self.height as u32,
        );

        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);

        encoder.write_header()?.write_image_data(&bytes)?;

        Ok(())
    }
}
