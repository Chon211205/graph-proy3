use crate::framebuffer::Framebuffer;

pub(super) fn draw_overlay(frame: &mut Framebuffer, color: u32, alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    for pixel in &mut frame.pixels {
        let channel = |shift: u32| {
            let source = ((*pixel >> shift) & 255) as f32;
            let overlay = ((color >> shift) & 255) as f32;
            (source + (overlay - source) * alpha).round() as u32
        };
        *pixel = channel(16) << 16 | channel(8) << 8 | channel(0);
    }
}

pub(super) fn panel(
    frame: &mut Framebuffer,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    color: u32,
    edge: u32,
) {
    let l = (left * frame.width as f32) as i32;
    let t = (top * frame.height as f32) as i32;
    let r = (right * frame.width as f32) as i32;
    let b = (bottom * frame.height as f32) as i32;
    rect(frame, l, t, r - l, b - t, color);
    rect(frame, l, t, r - l, 3, edge);
    rect(frame, l, b - 3, r - l, 3, edge);
    rect(frame, l, t, 3, b - t, edge);
    rect(frame, r - 3, t, 3, b - t, edge);
}

fn rect(frame: &mut Framebuffer, x: i32, y: i32, width: i32, height: i32, color: u32) {
    for py in y.max(0)..(y + height).min(frame.height as i32) {
        for px in x.max(0)..(x + width).min(frame.width as i32) {
            frame.pixels[py as usize * frame.width + px as usize] = color;
        }
    }
}

pub(super) fn bar(
    frame: &mut Framebuffer,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    ratio: f32,
    color: u32,
) {
    rect(frame, x, y, width, height, 0x251d2d);
    rect(
        frame,
        x,
        y,
        (width as f32 * ratio.clamp(0.0, 1.0)) as i32,
        height,
        color,
    );
}

pub(super) fn shield_icon(frame: &mut Framebuffer, x: i32, y: i32, size: i32, color: u32) {
    for row in 0..size {
        let width = if row < size * 2 / 3 {
            size * 2 / 3
        } else {
            size - (row - size * 2 / 3) * 2
        };
        let left = x + (size - width) / 2;
        rect(frame, left, y + row, width, 2.max(size / 7), color);
    }
}
