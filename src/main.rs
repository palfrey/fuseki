use std::time::Duration;

use gtp::Command;
use gtp::{controller::Engine, Response};
use libremarkable::{
    appctx,
    cgmath::{self, Point2},
    framebuffer::{
        common::{color, display_temp, dither_mode, mxcfb_rect, waveform_mode},
        core::Framebuffer,
        FramebufferDraw, FramebufferRefresh,
    },
    input::{InputEvent, MultitouchEvent},
};
use log::info;

const BOARD_SIZE: u8 = 9;
const SQUARE_COUNT: u8 = BOARD_SIZE - 1;
const AVAILABLE_WIDTH: u16 = libremarkable::dimensions::DISPLAYWIDTH - 200;
const SQUARE_SIZE: u16 = AVAILABLE_WIDTH / SQUARE_COUNT as u16;
const CIRCLE_RADIUS: u16 = ((SQUARE_SIZE as f64 / 2_f64) * 0.6) as u16;
const CIRCLE_BORDER: u16 = 5;
const SPARE_WIDTH: u16 =
    (libremarkable::dimensions::DISPLAYWIDTH - (SQUARE_SIZE * SQUARE_COUNT as u16)) / 2;
const SPARE_HEIGHT: u16 =
    (libremarkable::dimensions::DISPLAYHEIGHT - (SQUARE_SIZE * SQUARE_COUNT as u16)) / 2;
const BORDER_WIDTH: u32 = 10;

fn draw_piece(fb: &mut Framebuffer, x: u8, y: u8, white: bool) {
    // info!("draw_piece: {x} {y} {white}");
    let point = cgmath::Point2 {
        x: (SPARE_WIDTH + (SQUARE_SIZE * x as u16)) as i32,
        y: (SPARE_HEIGHT + (SQUARE_SIZE * y as u16)) as i32,
    };
    fb.fill_circle(point, CIRCLE_RADIUS as u32, color::BLACK);
    if white {
        fb.fill_circle(point, (CIRCLE_RADIUS - CIRCLE_BORDER) as u32, color::WHITE);
    }
}

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

fn do_human_move(ctrl: &mut Engine, pos: Point2<u8>) {
    let cmd = Command::new_with_args("play", |e| {
        e.s("white").v((pos.x as i32, pos.y as i32)).list()
    });
    info!("human: {}", cmd.to_string());
    ctrl.send(cmd);
    let resp = get_response(ctrl);
    info!("human resp: {}", resp.text());
}

fn refresh(fb: &mut Framebuffer) {
    fb.partial_refresh(
        &mxcfb_rect {
            top: 0,
            left: 0,
            width: libremarkable::dimensions::DISPLAYWIDTH as u32,
            height: libremarkable::dimensions::DISPLAYHEIGHT as u32,
        },
        libremarkable::framebuffer::PartialRefreshMode::Async,
        waveform_mode::WAVEFORM_MODE_AUTO,
        display_temp::TEMP_USE_REMARKABLE_DRAW,
        dither_mode::EPDC_FLAG_EXP1,
        0,
        false,
    );
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

fn list_stones(ctrl: &mut Engine, color: &str) -> Vec<gtp::Entity> {
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
    return ev.unwrap();
}

fn draw_stones(fb: &mut Framebuffer, ev: Vec<gtp::Entity>, white: bool) {
    for entity in ev {
        match entity {
            gtp::Entity::Vertex((x, y)) => {
                draw_piece(fb, x as u8, y as u8, white);
            }
            _ => {}
        }
    }
}

fn redraw_stones(ctrl: &mut Engine, fb: &mut Framebuffer) {
    draw_grid(fb);
    let white_stones = list_stones(ctrl, "white");
    draw_stones(fb, white_stones, true);
    let black_stones = list_stones(ctrl, "black");
    draw_stones(fb, black_stones, false);
    refresh(fb);
}

fn main() {
    env_logger::init();
    let mut app: appctx::ApplicationContext<'_> = appctx::ApplicationContext::default();

    let fb = app.get_framebuffer_ref();
    draw_grid(fb);

    info!("Starting GnuGo");
    let mut ctrl = Engine::new("./gnugo", &["--mode", "gtp"]);
    assert!(ctrl.start().is_ok());

    ctrl.send(Command::new_with_args("boardsize", |e| {
        e.i(BOARD_SIZE as u32)
    }));
    ctrl.wait_response(Duration::from_millis(500)).unwrap();

    do_machine_move(&mut ctrl);
    redraw_stones(&mut ctrl, fb);
    info!("Init complete. Beginning event dispatch...");

    // Blocking call to process events from digitizer + touchscreen + physical buttons
    app.start_event_loop(true, true, true, |ctx, evt| match evt {
        InputEvent::MultitouchEvent { event } => on_multitouch_event(ctx, event, &mut ctrl),
        ev => {
            info!("event: {ev:?}");
        }
    });
}

fn nearest_spot(x: u16, y: u16) -> Point2<u8> {
    Point2 {
        x: (((x - SPARE_WIDTH) as f32) / (SQUARE_SIZE as f32)).round() as u8,
        y: (((y - SPARE_HEIGHT) as f32) / (SQUARE_SIZE as f32)).round() as u8,
    }
}

fn on_multitouch_event(
    ctx: &mut appctx::ApplicationContext<'_>,
    event: MultitouchEvent,
    ctrl: &mut Engine,
) {
    match event {
        MultitouchEvent::Press { finger } => {
            let fb = ctx.get_framebuffer_ref();
            let point = nearest_spot(finger.pos.x, finger.pos.y);
            let pos = finger.pos;
            info!("Drawing: {point:?} for {pos:?}");
            do_human_move(ctrl, point);
            redraw_stones(ctrl, fb);
            do_machine_move(ctrl);
            redraw_stones(ctrl, fb);
        }
        _ => {}
    }
}
