use lazy_static::lazy_static;
use std::sync::Mutex;

use gtp::controller::Engine;
use libremarkable::{
    appctx,
    cgmath::{Point2, Vector2},
    framebuffer::{core::Framebuffer, FramebufferDraw},
    input::MultitouchEvent,
};

use crate::{
    board::{AVAILABLE_WIDTH, SPARE_WIDTH},
    drawing::{draw_text, refresh},
    routine::Routine,
};

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Mode {
    Chooser = 1,
    AgainstMachine = 2,
    Atari = 3,
    DragonGoServer = 4,
    Exit = 5,
}

pub static CURRENT_MODE: Mutex<Mode> = Mutex::new(Mode::Chooser);

struct Button {
    text: String,
    top_left: Point2<i32>,
    size: Vector2<u32>,
    mode: Mode,
}

const BUTTON_WIDTH: u32 = 700;
const TOP_LEFT_X: i32 =
    ((libremarkable::dimensions::DISPLAYWIDTH as u32 - BUTTON_WIDTH) / 2) as i32;

lazy_static! {
    static ref BUTTONS: Vec<Button> = {
        vec![
            Button {
                text: "Machine game".to_string(),
                top_left: Point2 {
                    x: TOP_LEFT_X,
                    y: 100,
                },
                size: Vector2 {
                    x: BUTTON_WIDTH,
                    y: 95,
                },
                mode: Mode::AgainstMachine,
            },
            Button {
                text: "Atari game".to_string(),
                top_left: Point2 {
                    x: TOP_LEFT_X,
                    y: 300,
                },
                size: Vector2 {
                    x: BUTTON_WIDTH,
                    y: 95,
                },
                mode: Mode::Atari,
            },
            Button {
                text: "Dragon Go Server".to_string(),
                top_left: Point2 {
                    x: TOP_LEFT_X,
                    y: 500,
                },
                size: Vector2 {
                    x: BUTTON_WIDTH,
                    y: 95,
                },
                mode: Mode::DragonGoServer,
            },
            Button {
                text: "Exit".to_string(),
                top_left: Point2 {
                    x: TOP_LEFT_X,
                    y: 700,
                },
                size: Vector2 {
                    x: BUTTON_WIDTH,
                    y: 95,
                },
                mode: Mode::Exit,
            },
        ]
    };
}

fn draw_chooser(fb: &mut Framebuffer) {
    fb.clear();
    for button in BUTTONS.iter() {
        draw_text(fb, &button.text, button.top_left, button.size);
    }
    refresh(fb);
}

fn on_multitouch_event(ctx: &mut appctx::ApplicationContext<'_>, event: MultitouchEvent) {
    match event {
        MultitouchEvent::Press { finger } => {
            for button in BUTTONS.iter() {
                if (finger.pos.x as i32) >= button.top_left.x
                    && (finger.pos.x as i32) < (button.top_left.x + button.size.x as i32)
                    && (finger.pos.y as i32) >= button.top_left.y
                    && (finger.pos.y as i32) < (button.top_left.y + button.size.y as i32)
                {
                    *CURRENT_MODE.lock().unwrap() = button.mode;
                    ctx.stop();
                    return;
                }
            }
        }
        _ => {}
    }
}

pub struct Chooser {}

impl Routine for Chooser {
    fn init(&self, fb: &mut Framebuffer, _ctrl: &mut Engine) {
        draw_chooser(fb);
    }

    fn on_multitouch_event(
        &self,
        ctx: &mut appctx::ApplicationContext<'_>,
        event: MultitouchEvent,
        _ctrl: &mut Engine,
    ) {
        on_multitouch_event(ctx, event);
    }
}
