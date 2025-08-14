use libremarkable::{
    cgmath::{Point2, Vector2},
    framebuffer::core::Framebuffer,
};

use crate::{
    board::{AVAILABLE_WIDTH, SPARE_WIDTH},
    drawing::draw_text,
};

pub const RESET_BUTTON_TOP_LEFT: Point2<i32> = Point2 {
    x: (SPARE_WIDTH + AVAILABLE_WIDTH / 2 - 10) as i32,
    y: 20,
};

pub const RESET_BUTTON_SIZE: Vector2<u32> = Vector2 { x: 500, y: 95 };

pub fn draw_reset(fb: &mut Framebuffer) {
    draw_text(fb, "Exit game", RESET_BUTTON_TOP_LEFT, RESET_BUTTON_SIZE);
}
