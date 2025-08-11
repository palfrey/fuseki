use std::sync::Mutex;
use std::time::{Duration, Instant};

use gtp::Command;
use gtp::{controller::Engine, Response};
use libremarkable::cgmath::Vector2;
use libremarkable::{
    appctx,
    cgmath::Point2,
    framebuffer::{
        common::{color, mxcfb_rect, waveform_mode},
        core::Framebuffer,
        FramebufferDraw,
    },
    input::{InputEvent, MultitouchEvent},
};
use log::info;

use crate::board::{draw_board, nearest_spot, AVAILABLE_WIDTH, BOARD_SIZE, SPARE_WIDTH};
use crate::drawing::{refresh, refresh_with_options};

mod board;
mod drawing;

#[derive(PartialEq, Debug, Clone, Copy)]
enum Turn {
    HumanTurn = 1,
    MachineTurn = 2,
}

static CURRENT_TURN: Mutex<Turn> = Mutex::new(Turn::MachineTurn);

fn get_response(ctrl: &mut Engine) -> Response {
    loop {
        match ctrl.wait_response(Duration::from_secs(1)) {
            Ok(resp) => {
                return resp;
            }
            Err(gtp::controller::Error::PollAgain) => continue,
            Err(err) => {
                panic!("Other error {err:?}")
            }
        }
    }
}

fn do_machine_move(ctrl: &mut Engine) {
    ctrl.send(Command::new_with_args("genmove", |e| e.s("black")));
    info!("waiting for machine response");
    let resp = get_response(ctrl);
    info!("machine: {}", resp.text());
}

fn do_human_move(ctrl: &mut Engine, pos: Point2<u8>) -> bool {
    let cmd = Command::new_with_args("play", |e| {
        e.s("white")
            .v(((pos.x + 1) as i32, (pos.y + 1) as i32))
            .list()
    });
    info!("human: {}", cmd.to_string());
    ctrl.send(cmd);
    let resp = get_response(ctrl);
    info!("human resp: {}", resp.text());
    return resp.text() == "";
}

fn list_stones(ctrl: &mut Engine, color: &str) -> Vec<gtp::Entity> {
    let start = Instant::now();
    let cmd = Command::new_with_args("list_stones", |e| e.s(color));
    // info!("list_stones: {}", cmd.to_string());
    ctrl.send(cmd);
    let resp = get_response(ctrl);
    // info!("list_stones resp: {}", resp.text());
    let ev = resp.entities(|ep| {
        let mut ret = ep;
        while !ret.is_eof() {
            ret = ret.vertex();
        }
        ret
    });
    let elapsed = start.elapsed();
    info!("list_stones elapsed: {:.2?}", elapsed);
    return ev.unwrap();
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

const RESET_BUTTON_TOP_LEFT: Point2<i32> = Point2 {
    x: (SPARE_WIDTH + AVAILABLE_WIDTH / 2 - 10) as i32,
    y: 20,
};

const RESET_BUTTON_SIZE: Vector2<u32> = Vector2 { x: 500, y: 95 };

fn draw_reset(fb: &mut Framebuffer) {
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

fn set_turn(turn: Turn, fb: &mut Framebuffer) {
    info!("Set turn {turn:?}");
    *CURRENT_TURN.lock().unwrap() = turn;
    draw_turn(fb, true);
}

fn reset_game(ctrl: &mut Engine, fb: &mut Framebuffer) {
    ctrl.send(Command::new_with_args("clear_board", |e| e));
    let resp = get_response(ctrl);
    info!("clear_board: {}", resp.text());

    do_machine_move(ctrl);
    redraw_stones(ctrl, fb);
}

fn main() {
    env_logger::init();
    let mut app: appctx::ApplicationContext<'_> = appctx::ApplicationContext::default();

    let fb = app.get_framebuffer_ref();

    info!("Starting GnuGo");
    let mut ctrl = Engine::new("./gnugo", &["--mode", "gtp", "--level", "8"]);
    assert!(ctrl.start().is_ok());

    ctrl.send(Command::new_with_args("boardsize", |e| {
        e.i(BOARD_SIZE as u32)
    }));
    ctrl.wait_response(Duration::from_millis(500)).unwrap();
    reset_game(&mut ctrl, fb);

    info!("Init complete. Beginning event dispatch...");

    set_turn(Turn::HumanTurn, fb);

    // Blocking call to process events from digitizer + touchscreen + physical buttons
    app.start_event_loop(true, true, true, |ctx, evt| match evt {
        InputEvent::MultitouchEvent { event } => on_multitouch_event(ctx, event, &mut ctrl),
        ev => {
            info!("event: {ev:?}");
        }
    });
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
                reset_game(ctrl, fb);
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
