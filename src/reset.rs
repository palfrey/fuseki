use libremarkable::{
    cgmath::{Point2, Vector2},
    framebuffer::{common::color, core::Framebuffer, FramebufferDraw},
};

use crate::board::{AVAILABLE_WIDTH, SPARE_WIDTH};

pub const RESET_BUTTON_TOP_LEFT: Point2<i32> = Point2 {
    x: (SPARE_WIDTH + AVAILABLE_WIDTH / 2 - 10) as i32,
    y: 20,
};

pub const RESET_BUTTON_SIZE: Vector2<u32> = Vector2 { x: 500, y: 95 };

pub fn draw_reset(fb: &mut Framebuffer) {
    fb.draw_rect(RESET_BUTTON_TOP_LEFT, RESET_BUTTON_SIZE, 5, color::BLACK);
    fb.draw_text(
        Point2 {
            x: (SPARE_WIDTH + AVAILABLE_WIDTH / 2) as f32,
            y: 100.0,
        },
        "Reset game",
        100.0,
        color::BLACK,
        false,
    );
}
