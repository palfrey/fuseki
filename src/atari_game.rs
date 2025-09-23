use crate::{
    board::{Board, AVAILABLE_WIDTH},
    chooser::CURRENT_MODE,
    drawing::{draw_text, refresh, refresh_with_options},
    gtp::{clear_board, count_captures, do_human_move, list_stones, set_board_size, undo_move},
    reset::{draw_reset, reset_button_top_left},
    routine::Routine,
};
use gtp::controller::Engine;
use libremarkable::{
    appctx,
    cgmath::{Point2, Vector2},
    framebuffer::{
        common::{color, mxcfb_rect, waveform_mode},
        core::Framebuffer,
        FramebufferDraw,
    },
    input::MultitouchEvent,
};
use log::info;
use std::time::Instant;

#[derive(PartialEq, Debug, Clone, Copy)]
enum Turn {
    WhiteTurn = 1,
    BlackTurn = 2,
}

pub struct AtariGame {
    board: Board,
    current_turn: Turn,
    game_end: Option<Turn>,
    undo_button_top_left: Point2<i32>,
}

pub const UNDO_BUTTON_SIZE: Vector2<u32> = Vector2 { x: 350, y: 95 };

impl AtariGame {
    pub fn new() -> AtariGame {
        let board: Board = Board::new(9);
        let undo_button_top_left = Point2 {
            x: (board.spare_width + AVAILABLE_WIDTH / 2 - 10) as i32,
            y: 120,
        };
        AtariGame {
            board,
            current_turn: Turn::BlackTurn,
            game_end: None,
            undo_button_top_left,
        }
    }

    fn set_turn(&mut self, turn: Turn, fb: &mut Framebuffer) {
        info!("Set turn {turn:?}");
        self.current_turn = turn;
        self.draw_turn(fb, true);
    }

    fn draw_turn(&self, fb: &mut Framebuffer, refresh: bool) {
        info!("draw_turn {:?}", self.current_turn);
        let text = if self.current_turn == Turn::WhiteTurn {
            "White turn"
        } else {
            "Black turn"
        };
        self.draw_status(fb, text, refresh);
    }

    fn draw_status(&self, fb: &mut Framebuffer, text: &str, refresh: bool) {
        let rect_width = 550;
        fb.fill_rect(
            Point2 {
                x: self.board.spare_width as i32,
                y: 0,
            },
            Vector2 {
                x: rect_width,
                y: 100,
            },
            color::WHITE,
        );
        fb.draw_text(
            Point2 {
                x: self.board.spare_width as f32,
                y: 100.0,
            },
            text,
            100.0,
            color::BLACK,
            false,
        );

        draw_text(fb, "Undo", self.undo_button_top_left, UNDO_BUTTON_SIZE);

        if refresh {
            refresh_with_options(
                fb,
                &mxcfb_rect {
                    top: 0,
                    left: self.board.spare_width as u32,
                    width: rect_width,
                    height: 100,
                },
                waveform_mode::WAVEFORM_MODE_AUTO,
            );
        }
    }

    fn reset_game(&self, ctrl: &mut Engine, fb: &mut Framebuffer) {
        clear_board(ctrl);
        self.redraw_stones(ctrl, fb);
    }

    fn draw_game_state(&self, fb: &mut Framebuffer) {
        match self.game_end {
            None => self.draw_turn(fb, false),
            Some(Turn::WhiteTurn) => self.draw_status(fb, "White win!", true),
            Some(Turn::BlackTurn) => self.draw_status(fb, "Black win!", true),
        }
    }

    fn redraw_stones(&self, ctrl: &mut Engine, fb: &mut Framebuffer) {
        let start = Instant::now();
        let white_stones = list_stones(ctrl, "white");
        let black_stones = list_stones(ctrl, "black");
        self.board.draw_board(fb, &white_stones, &black_stones);
        self.draw_game_state(fb);
        draw_reset(&self.board, fb);
        refresh(fb);
        let elapsed = start.elapsed();
        info!("redraw elapsed: {:.2?}", elapsed);
    }
}

impl Routine for AtariGame {
    fn init(&mut self, fb: &mut Framebuffer, ctrl: &mut Engine) {
        set_board_size(ctrl, self.board.board_size);
        self.set_turn(Turn::BlackTurn, fb);
        self.reset_game(ctrl, fb);
    }

    fn on_multitouch_event(
        &mut self,
        ctx: &mut appctx::ApplicationContext<'_>,
        event: MultitouchEvent,
        ctrl: &mut Engine,
    ) {
        match event {
            MultitouchEvent::Press { finger } => {
                let start = Instant::now();
                let fb = ctx.get_framebuffer_ref();

                let rbtl = reset_button_top_left(&self.board);
                if (finger.pos.x as i32) >= rbtl.x
                    && (finger.pos.x as i32) < (rbtl.x + rbtl.x as i32)
                    && (finger.pos.y as i32) >= rbtl.y
                    && (finger.pos.y as i32) < (rbtl.y + rbtl.y as i32)
                {
                    *CURRENT_MODE.lock().unwrap() = crate::chooser::Mode::Chooser;
                    ctx.stop();
                    return;
                }

                if (finger.pos.x as i32) >= self.undo_button_top_left.x
                    && (finger.pos.x as i32)
                        < (self.undo_button_top_left.x + UNDO_BUTTON_SIZE.x as i32)
                    && (finger.pos.y as i32) >= self.undo_button_top_left.y
                    && (finger.pos.y as i32)
                        < (self.undo_button_top_left.y + UNDO_BUTTON_SIZE.y as i32)
                {
                    if undo_move(ctrl) {
                        match self.current_turn {
                            Turn::WhiteTurn => self.set_turn(Turn::BlackTurn, fb),
                            Turn::BlackTurn => self.set_turn(Turn::WhiteTurn, fb),
                        }
                        self.redraw_stones(ctrl, fb);
                    }
                    return;
                }

                let point = self.board.nearest_spot(finger.pos.x, finger.pos.y);
                let pos = finger.pos;
                if point.x >= self.board.board_size || point.y >= self.board.board_size {
                    info!("Bad point {point:?}");
                    return;
                }
                info!("Drawing: {point:?} for {pos:?}");

                match self.current_turn {
                    Turn::WhiteTurn => {
                        if !do_human_move(ctrl, point, "white") {
                            info!("Bad white move");
                            return;
                        }
                        if count_captures(ctrl, "white") > 0 {
                            info!("White win");

                            self.game_end = Some(Turn::WhiteTurn);
                            self.redraw_stones(ctrl, fb);
                        } else {
                            self.set_turn(Turn::BlackTurn, fb);
                            self.board
                                .refresh_and_draw_one_piece(fb, point.x, point.y, true);
                        }
                    }
                    Turn::BlackTurn => {
                        if !do_human_move(ctrl, point, "black") {
                            info!("Bad black move");
                            return;
                        }
                        if count_captures(ctrl, "black") > 0 {
                            info!("Black win");
                            self.game_end = Some(Turn::BlackTurn);
                            self.redraw_stones(ctrl, fb);
                        } else {
                            self.set_turn(Turn::WhiteTurn, fb);
                            self.board
                                .refresh_and_draw_one_piece(fb, point.x, point.y, false);
                        }
                    }
                };

                let elapsed = start.elapsed();
                info!("touch elapsed: {:.2?}", elapsed);
            }
            _ => {}
        }
    }
}
