use gtp::Entity;
use libremarkable::{
    cgmath::{self, Point2},
    framebuffer::{
        common::{color, mxcfb_rect, waveform_mode},
        core::Framebuffer,
        FramebufferDraw,
    },
};

use crate::drawing::refresh_with_options;

pub const BOARD_SIZE: u8 = 9;
const SQUARE_COUNT: u8 = BOARD_SIZE - 1;
pub const AVAILABLE_WIDTH: u16 = libremarkable::dimensions::DISPLAYWIDTH - 200;
const SQUARE_SIZE: u16 = AVAILABLE_WIDTH / SQUARE_COUNT as u16;
const CIRCLE_RADIUS: u16 = ((SQUARE_SIZE as f64 / 2_f64) * 0.6) as u16;
const CIRCLE_BORDER: u16 = 5;
pub const SPARE_WIDTH: u16 =
    (libremarkable::dimensions::DISPLAYWIDTH - (SQUARE_SIZE * SQUARE_COUNT as u16)) / 2;
pub const SPARE_HEIGHT: u16 =
    (libremarkable::dimensions::DISPLAYHEIGHT - (SQUARE_SIZE * SQUARE_COUNT as u16)) / 2;
const BORDER_WIDTH: u32 = 10;

fn draw_piece(fb: &mut Framebuffer, x: u8, y: u8, white: bool) -> mxcfb_rect {
    // info!("draw_piece: {x} {y} {white}");
    let point = Point2 {
        x: (SPARE_WIDTH + (SQUARE_SIZE * x as u16)) as i32,
        y: (SPARE_HEIGHT + (SQUARE_SIZE * y as u16)) as i32,
    };
    let rect = fb.fill_circle(point, CIRCLE_RADIUS as u32, color::BLACK);
    if white {
        fb.fill_circle(point, (CIRCLE_RADIUS - CIRCLE_BORDER) as u32, color::WHITE);
    }
    rect
}

pub fn refresh_and_draw_one_piece(fb: &mut Framebuffer, x: u8, y: u8, white: bool) {
    let rect = draw_piece(fb, x, y, white);
    refresh_with_options(fb, &rect, waveform_mode::WAVEFORM_MODE_AUTO);
}

fn draw_grid(fb: &mut Framebuffer) {
    fb.clear();

    for y in 0..SQUARE_COUNT {
        for x in 0..SQUARE_COUNT {
            fb.draw_rect(
                cgmath::Point2 {
                    x: (SQUARE_SIZE * (x as u16) + SPARE_WIDTH) as i32,
                    y: (SQUARE_SIZE * (y as u16) + SPARE_HEIGHT) as i32,
                },
                cgmath::Vector2 {
                    x: SQUARE_SIZE as u32,
                    y: SQUARE_SIZE as u32,
                },
                BORDER_WIDTH,
                color::BLACK,
            );
        }
    }
}

fn draw_stones(fb: &mut Framebuffer, ev: Vec<gtp::Entity>, white: bool) {
    for entity in ev {
        match entity {
            gtp::Entity::Vertex((x, y)) => {
                draw_piece(fb, (x - 1) as u8, (y - 1) as u8, white);
            }
            _ => {}
        }
    }
}

pub fn nearest_spot(x: u16, y: u16) -> Point2<u8> {
    let raw_point = Point2::<f32> {
        x: (((x - SPARE_WIDTH) as f32) / (SQUARE_SIZE as f32)).round(),
        y: (((y - SPARE_HEIGHT) as f32) / (SQUARE_SIZE as f32)).round(),
    };
    if raw_point.x < 0.0 || raw_point.y < 0.0 {
        return Point2 {
            x: BOARD_SIZE,
            y: BOARD_SIZE,
        };
    } else {
        Point2 {
            x: raw_point.x as u8,
            y: raw_point.y as u8,
        }
    }
}

pub fn draw_board(fb: &mut Framebuffer, white_stones: Vec<Entity>, black_stones: Vec<Entity>) {
    draw_grid(fb);
    draw_stones(fb, white_stones, true);
    draw_stones(fb, black_stones, false);
}
