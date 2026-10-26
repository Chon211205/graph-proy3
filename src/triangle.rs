
use crate::framebuffer::Framebuffer;

#[derive(Clone, Copy)]
pub struct ScreenVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

fn edge(
    a: ScreenVertex,
    b: ScreenVertex,
    x: f32,
    y: f32,
) -> f32 {
    (x - a.x) * (b.y - a.y)
        - (y - a.y) * (b.x - a.x)
}

pub fn draw_triangle(
    framebuffer: &mut Framebuffer,
    a: ScreenVertex,
    b: ScreenVertex,
    c: ScreenVertex,
    color: u32,
) {
    let area = edge(a, b, c.x, c.y);

    if area.abs() < 1e-7 {
        return;
    }

    let min_x = (a.x.min(b.x).min(c.x).floor() as i32).max(0);
    let max_x = (a.x.max(b.x).max(c.x).ceil() as i32)
        .min(framebuffer.width as i32 - 1);

    let min_y = (a.y.min(b.y).min(c.y).floor() as i32).max(0);
    let max_y = (a.y.max(b.y).max(c.y).ceil() as i32)
        .min(framebuffer.height as i32 - 1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;

            let w0 = edge(b, c, px, py) / area;
            let w1 = edge(c, a, px, py) / area;
            let w2 = 1.0 - w0 - w1;

            if w0 >= -1e-6
                && w1 >= -1e-6
                && w2 >= -1e-6
            {
                let z = w0 * a.z
                    + w1 * b.z
                    + w2 * c.z;

                framebuffer.pixel(x, y, z, color);
            }
        }
    }
}
