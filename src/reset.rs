use libremarkable::{
    cgmath::{Point2, Vector2},
    framebuffer::core::Framebuffer,
};

use crate::{
    board::{Board, AVAILABLE_WIDTH},
    drawing::draw_button,
};

pub const RESET_BUTTON_SIZE: Vector2<u32> = Vector2 { x: 500, y: 95 };

pub fn reset_button_top_left(board: &Board) -> Point2<i32> {
    Point2 {
        x: (board.spare_width + AVAILABLE_WIDTH / 2 - 10) as i32,
        y: 20,
    }
}

pub fn draw_reset(board: &Board, fb: &mut Framebuffer) {
    draw_button(
        fb,
        "Exit game",
        reset_button_top_left(board),
        RESET_BUTTON_SIZE,
    );
}
