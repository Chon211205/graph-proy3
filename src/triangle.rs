use crate::framebuffer::Framebuffer;

#[derive(Clone, Copy)]
pub struct ScreenVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

fn edge(a: ScreenVertex, b: ScreenVertex, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

pub fn draw_triangle(
    framebuffer: &mut Framebuffer,
    a: ScreenVertex,
    b: ScreenVertex,
    c: ScreenVertex,
    color: u32,
) {
    draw_triangle_shaded(framebuffer, a, b, c, |_, _, _, _, _| color);
}

pub fn draw_triangle_shaded<F>(
    framebuffer: &mut Framebuffer,
    a: ScreenVertex,
    b: ScreenVertex,
    c: ScreenVertex,
    shader: F,
) where
    F: Fn(f32, f32, f32, i32, i32) -> u32,
{
    let area = edge(a, b, c.x, c.y);

    if area.abs() < 1e-7 {
        return;
    }

    let min_x = (a.x.min(b.x).min(c.x).floor() as i32).max(0);
    let max_x = (a.x.max(b.x).max(c.x).ceil() as i32).min(framebuffer.width as i32 - 1);

    let min_y = (a.y.min(b.y).min(c.y).floor() as i32).max(0);
    let max_y = (a.y.max(b.y).max(c.y).ceil() as i32).min(framebuffer.height as i32 - 1);

    if min_x > max_x || min_y > max_y {
        return;
    }

    // Edge equations are linear. Evaluate them once and advance with additions
    // instead of doing six subtractions, four multiplications and two divisions
    // for every candidate pixel.
    let inverse_area = 1.0 / area;
    let start_x = min_x as f32 + 0.5;
    let start_y = min_y as f32 + 0.5;
    let w0_dx = (c.y - b.y) * inverse_area;
    let w0_dy = -(c.x - b.x) * inverse_area;
    let w1_dx = (a.y - c.y) * inverse_area;
    let w1_dy = -(a.x - c.x) * inverse_area;
    let mut row_w0 = edge(b, c, start_x, start_y) * inverse_area;
    let mut row_w1 = edge(c, a, start_x, start_y) * inverse_area;

    for y in min_y..=max_y {
        let mut w0 = row_w0;
        let mut w1 = row_w1;
        for x in min_x..=max_x {
            let w2 = 1.0 - w0 - w1;

            if w0 >= -1e-6 && w1 >= -1e-6 && w2 >= -1e-6 {
                let z = w0 * a.z + w1 * b.z + w2 * c.z;

                let color = shader(w0, w1, w2, x, y);
                framebuffer.pixel_clipped(x as usize, y as usize, z, color);
            }
            w0 += w0_dx;
            w1 += w1_dx;
        }
        row_w0 += w0_dy;
        row_w1 += w1_dy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizer_draws_inside_but_not_outside_triangle() {
        let mut framebuffer = Framebuffer::new(8, 8);
        draw_triangle(
            &mut framebuffer,
            ScreenVertex {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            ScreenVertex {
                x: 6.0,
                y: 1.0,
                z: 1.0,
            },
            ScreenVertex {
                x: 1.0,
                y: 6.0,
                z: 1.0,
            },
            0xff00ff,
        );
        assert_eq!(framebuffer.pixels[2 * 8 + 2], 0xff00ff);
        assert_eq!(framebuffer.pixels[7 * 8 + 7], 0);
    }

    #[test]
    fn rasterizer_clips_triangles_at_framebuffer_edges() {
        let mut framebuffer = Framebuffer::new(4, 4);
        draw_triangle(
            &mut framebuffer,
            ScreenVertex {
                x: -10.0,
                y: -10.0,
                z: 1.0,
            },
            ScreenVertex {
                x: 3.0,
                y: 0.0,
                z: 1.0,
            },
            ScreenVertex {
                x: 0.0,
                y: 3.0,
                z: 1.0,
            },
            0xffffff,
        );
        assert!(framebuffer.pixels.iter().any(|&pixel| pixel != 0));
    }
}
