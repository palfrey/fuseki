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

pub struct Board {
    pub board_size: u8,
    pub square_count: u8,
    pub square_size: u16,
    pub circle_radius: u16,
    pub spare_width: u16,
    pub spare_height: u16,
}

pub const AVAILABLE_WIDTH: u16 = libremarkable::dimensions::DISPLAYWIDTH - 200;
const CIRCLE_BORDER: u16 = 5;
const BORDER_WIDTH: u32 = 10;

impl Board {
    pub fn new(board_size: u8) -> Board {
        let square_count = board_size - 1;
        let square_size = AVAILABLE_WIDTH / square_count as u16;
        Board {
            board_size,
            square_count,
            square_size,
            circle_radius: ((square_size as f64 / 2_f64) * 0.6) as u16,
            spare_width: (libremarkable::dimensions::DISPLAYWIDTH
                - (square_size * square_count as u16))
                / 2,
            spare_height: (libremarkable::dimensions::DISPLAYHEIGHT
                - (square_size * square_count as u16))
                / 2,
        }
    }

    fn draw_piece(&self, fb: &mut Framebuffer, x: u8, y: u8, white: bool) -> mxcfb_rect {
        // info!("draw_piece: {x} {y} {white}");
        let point = Point2 {
            x: (self.spare_width + (self.square_size * x as u16)) as i32,
            y: (self.spare_height + (self.square_size * y as u16)) as i32,
        };
        let rect = fb.fill_circle(point, self.circle_radius as u32, color::BLACK);
        if white {
            fb.fill_circle(
                point,
                (self.circle_radius - CIRCLE_BORDER) as u32,
                color::WHITE,
            );
        }
        rect
    }

    pub fn refresh_and_draw_one_piece(&self, fb: &mut Framebuffer, x: u8, y: u8, white: bool) {
        let rect = self.draw_piece(fb, x, y, white);
        refresh_with_options(fb, &rect, waveform_mode::WAVEFORM_MODE_AUTO);
    }

    fn draw_grid(&self, fb: &mut Framebuffer) {
        fb.clear();

        for y in 0..self.square_count {
            for x in 0..self.square_count {
                fb.draw_rect(
                    cgmath::Point2 {
                        x: (self.square_size * (x as u16) + self.spare_width) as i32,
                        y: (self.square_size * (y as u16) + self.spare_height) as i32,
                    },
                    cgmath::Vector2 {
                        x: self.square_size as u32,
                        y: self.square_size as u32,
                    },
                    BORDER_WIDTH,
                    color::BLACK,
                );
            }
        }
    }

    fn draw_stones(&self, fb: &mut Framebuffer, ev: &Vec<gtp::Entity>, white: bool) {
        for entity in ev {
            match entity {
                gtp::Entity::Vertex((x, y)) => {
                    self.draw_piece(fb, (x - 1) as u8, (y - 1) as u8, white);
                }
                _ => {}
            }
        }
    }

    pub fn nearest_spot(&self, x: u16, y: u16) -> Point2<u8> {
        let raw_point = Point2::<f32> {
            x: (((x - self.spare_width) as f32) / (self.square_size as f32)).round(),
            y: (((y - self.spare_height) as f32) / (self.square_size as f32)).round(),
        };
        if raw_point.x < 0.0 || raw_point.y < 0.0 {
            return Point2 {
                x: self.board_size,
                y: self.board_size,
            };
        } else {
            Point2 {
                x: raw_point.x as u8,
                y: raw_point.y as u8,
            }
        }
    }

    pub fn draw_board(
        &self,
        fb: &mut Framebuffer,
        white_stones: &Vec<Entity>,
        black_stones: &Vec<Entity>,
    ) {
        self.draw_grid(fb);
        self.draw_stones(fb, white_stones, true);
        self.draw_stones(fb, black_stones, false);
    }
}
