use std::{sync::Mutex, time::Instant};

use gtp::{controller::Engine, Command};
use libremarkable::{
    appctx,
    cgmath::{Point2, Vector2},
    framebuffer::{
        common::{color, mxcfb_rect, waveform_mode},
        core::Framebuffer,
        FramebufferDraw,
    },
    input::{InputEvent, MultitouchEvent},
};
use log::info;

use crate::{
    board::{draw_board, nearest_spot, BOARD_SIZE, SPARE_WIDTH},
    drawing::{refresh, refresh_with_options},
    gtp::{clear_board, do_human_move, get_response, list_stones},
    reset::{draw_reset, RESET_BUTTON_SIZE, RESET_BUTTON_TOP_LEFT},
};

#[derive(PartialEq, Debug, Clone, Copy)]
enum Turn {
    HumanTurn = 1,
    MachineTurn = 2,
}

static CURRENT_TURN: Mutex<Turn> = Mutex::new(Turn::MachineTurn);

fn set_turn(turn: Turn, fb: &mut Framebuffer) {
    info!("Set turn {turn:?}");
    *CURRENT_TURN.lock().unwrap() = turn;
    draw_turn(fb, true);
}

fn do_machine_move(ctrl: &mut Engine) {
    ctrl.send(Command::new_with_args("genmove", |e| e.s("black")));
    info!("waiting for machine response");
    let resp = get_response(ctrl);
    info!("machine: {}", resp.text());
}

fn draw_turn(fb: &mut Framebuffer, refresh: bool) {
    let rect_width = 550;
    let turn: Turn = *CURRENT_TURN.lock().unwrap();
    info!("draw_turn {turn:?}");
    let text = if turn == Turn::HumanTurn {
        "Human turn"
    } else {
        "Machine turn"
    };
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

fn reset_machine_game(ctrl: &mut Engine, fb: &mut Framebuffer) {
    clear_board(ctrl);
    do_machine_move(ctrl);
    redraw_stones(ctrl, fb);
}

fn redraw_stones(ctrl: &mut Engine, fb: &mut Framebuffer) {
    let start = Instant::now();
    let white_stones = list_stones(ctrl, "white");
    let black_stones = list_stones(ctrl, "black");
    draw_board(fb, white_stones, black_stones);
    draw_turn(fb, false);
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
            if *CURRENT_TURN.lock().unwrap() != Turn::HumanTurn {
                info!("Ignoring touch, as machine turn");
                return;
            }
            let fb = ctx.get_framebuffer_ref();

            if (finger.pos.x as i32) >= RESET_BUTTON_TOP_LEFT.x
                && (finger.pos.x as i32) < (RESET_BUTTON_TOP_LEFT.x + RESET_BUTTON_SIZE.x as i32)
                && (finger.pos.y as i32) >= RESET_BUTTON_TOP_LEFT.y
                && (finger.pos.y as i32) < (RESET_BUTTON_TOP_LEFT.y + RESET_BUTTON_SIZE.y as i32)
            {
                reset_machine_game(ctrl, fb);
                return;
            }

            let point = nearest_spot(finger.pos.x, finger.pos.y);
            let pos = finger.pos;
            if point.x >= BOARD_SIZE || point.y >= BOARD_SIZE {
                info!("Bad point {point:?}");
                return;
            }
            info!("Drawing: {point:?} for {pos:?}");
            if !do_human_move(ctrl, point) {
                info!("Bad human move");
                return;
            }
            set_turn(Turn::MachineTurn, fb);
            redraw_stones(ctrl, fb);
            do_machine_move(ctrl);
            redraw_stones(ctrl, fb);
            set_turn(Turn::HumanTurn, fb);
        }
        _ => {}
    }
}

pub fn run_game(ctrl: &mut Engine, fb: &mut Framebuffer, app: &mut appctx::ApplicationContext<'_>) {
    reset_machine_game(ctrl, fb);
    set_turn(Turn::HumanTurn, fb);
    app.start_event_loop(true, true, true, |ctx, evt| match evt {
        InputEvent::MultitouchEvent { event } => on_multitouch_event(ctx, event, ctrl),
        ev => {
            info!("event: {ev:?}");
        }
    });
}
