use std::{sync::Mutex, time::Instant};

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

use crate::{
    board::{
        draw_board, nearest_spot, refresh_and_draw_one_piece, AVAILABLE_WIDTH, BOARD_SIZE,
        SPARE_WIDTH,
    },
    chooser::CURRENT_MODE,
    drawing::{draw_text, refresh, refresh_with_options},
    gtp::{clear_board, count_captures, do_human_move, list_stones, undo_move},
    reset::{draw_reset, RESET_BUTTON_SIZE, RESET_BUTTON_TOP_LEFT},
    routine::Routine,
};

#[derive(PartialEq, Debug, Clone, Copy)]
enum Turn {
    WhiteTurn = 1,
    BlackTurn = 2,
}

static CURRENT_TURN: Mutex<Turn> = Mutex::new(Turn::BlackTurn);
static GAME_END: Mutex<Option<Turn>> = Mutex::new(None);

fn set_turn(turn: Turn, fb: &mut Framebuffer) {
    info!("Set turn {turn:?}");
    *CURRENT_TURN.lock().unwrap() = turn;
    draw_turn(fb, true);
}

fn draw_turn(fb: &mut Framebuffer, refresh: bool) {
    let turn: Turn = *CURRENT_TURN.lock().unwrap();
    info!("draw_turn {turn:?}");
    let text = if turn == Turn::WhiteTurn {
        "White turn"
    } else {
        "Black turn"
    };
    draw_status(fb, text, refresh);
}

pub const UNDO_BUTTON_TOP_LEFT: Point2<i32> = Point2 {
    x: (SPARE_WIDTH + AVAILABLE_WIDTH / 2 - 10) as i32,
    y: 120,
};

pub const UNDO_BUTTON_SIZE: Vector2<u32> = Vector2 { x: 350, y: 95 };

fn draw_status(fb: &mut Framebuffer, text: &str, refresh: bool) {
    let rect_width = 550;
    fb.fill_rect(
        Point2 {
            x: SPARE_WIDTH as i32,
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
            x: SPARE_WIDTH as f32,
            y: 100.0,
        },
        text,
        100.0,
        color::BLACK,
        false,
    );

    draw_text(fb, "Undo", UNDO_BUTTON_TOP_LEFT, UNDO_BUTTON_SIZE);

    if refresh {
        refresh_with_options(
            fb,
            &mxcfb_rect {
                top: 0,
                left: SPARE_WIDTH as u32,
                width: rect_width,
                height: 100,
            },
            waveform_mode::WAVEFORM_MODE_AUTO,
        );
    }
}

fn reset_atari_game(ctrl: &mut Engine, fb: &mut Framebuffer) {
    clear_board(ctrl);
    redraw_stones(ctrl, fb);
}

fn draw_game_state(fb: &mut Framebuffer) {
    match *GAME_END.lock().unwrap() {
        None => draw_turn(fb, false),
        Some(Turn::WhiteTurn) => draw_status(fb, "White win!", true),
        Some(Turn::BlackTurn) => draw_status(fb, "Black win!", true),
    }
}

fn redraw_stones(ctrl: &mut Engine, fb: &mut Framebuffer) {
    let start = Instant::now();
    let white_stones = list_stones(ctrl, "white");
    let black_stones = list_stones(ctrl, "black");
    draw_board(fb, white_stones, black_stones);
    draw_game_state(fb);
    draw_reset(fb);
    refresh(fb);
    let elapsed = start.elapsed();
    info!("redraw elapsed: {:.2?}", elapsed);
}

fn on_multitouch_event(
    ctx: &mut appctx::ApplicationContext<'_>,
    event: MultitouchEvent,
    ctrl: &mut Engine,
) {
    match event {
        MultitouchEvent::Press { finger } => {
            let start = Instant::now();
            let fb = ctx.get_framebuffer_ref();

            if (finger.pos.x as i32) >= RESET_BUTTON_TOP_LEFT.x
                && (finger.pos.x as i32) < (RESET_BUTTON_TOP_LEFT.x + RESET_BUTTON_SIZE.x as i32)
                && (finger.pos.y as i32) >= RESET_BUTTON_TOP_LEFT.y
                && (finger.pos.y as i32) < (RESET_BUTTON_TOP_LEFT.y + RESET_BUTTON_SIZE.y as i32)
            {
                *CURRENT_MODE.lock().unwrap() = crate::chooser::Mode::Chooser;
                ctx.stop();
                return;
            }

            if (finger.pos.x as i32) >= UNDO_BUTTON_TOP_LEFT.x
                && (finger.pos.x as i32) < (UNDO_BUTTON_TOP_LEFT.x + UNDO_BUTTON_SIZE.x as i32)
                && (finger.pos.y as i32) >= UNDO_BUTTON_TOP_LEFT.y
                && (finger.pos.y as i32) < (UNDO_BUTTON_TOP_LEFT.y + UNDO_BUTTON_SIZE.y as i32)
            {
                if undo_move(ctrl) {
                    let current_turn = *CURRENT_TURN.lock().unwrap();
                    match current_turn {
                        Turn::WhiteTurn => set_turn(Turn::BlackTurn, fb),
                        Turn::BlackTurn => set_turn(Turn::WhiteTurn, fb),
                    }
                    redraw_stones(ctrl, fb);
                }
                return;
            }

            let point = nearest_spot(finger.pos.x, finger.pos.y);
            let pos = finger.pos;
            if point.x >= BOARD_SIZE || point.y >= BOARD_SIZE {
                info!("Bad point {point:?}");
                return;
            }
            info!("Drawing: {point:?} for {pos:?}");

            let current_turn = *CURRENT_TURN.lock().unwrap();
            match current_turn {
                Turn::WhiteTurn => {
                    if !do_human_move(ctrl, point, "white") {
                        info!("Bad white move");
                        return;
                    }
                    if count_captures(ctrl, "white") > 0 {
                        info!("White win");

                        *GAME_END.lock().unwrap() = Some(Turn::WhiteTurn);
                        redraw_stones(ctrl, fb);
                    } else {
                        set_turn(Turn::BlackTurn, fb);
                        refresh_and_draw_one_piece(fb, point.x, point.y, true);
                    }
                }
                Turn::BlackTurn => {
                    if !do_human_move(ctrl, point, "black") {
                        info!("Bad black move");
                        return;
                    }
                    if count_captures(ctrl, "black") > 0 {
                        info!("Black win");
                        *GAME_END.lock().unwrap() = Some(Turn::BlackTurn);
                        redraw_stones(ctrl, fb);
                    } else {
                        set_turn(Turn::WhiteTurn, fb);
                        refresh_and_draw_one_piece(fb, point.x, point.y, false);
                    }
                }
            };

            let elapsed = start.elapsed();
            info!("touch elapsed: {:.2?}", elapsed);
        }
        _ => {}
    }
}

pub struct AtariGame {}

impl Routine for AtariGame {
    fn init(&self, fb: &mut Framebuffer, ctrl: &mut Engine) {
        set_turn(Turn::BlackTurn, fb);
        reset_atari_game(ctrl, fb);
    }

    fn on_multitouch_event(
        &self,
        ctx: &mut appctx::ApplicationContext<'_>,
        event: MultitouchEvent,
        ctrl: &mut Engine,
    ) {
        on_multitouch_event(ctx, event, ctrl);
    }
}
