use crate::{
    board::Board,
    chooser::CURRENT_MODE,
    drawing::{refresh, refresh_with_options},
    gtp::{clear_board, do_human_move, get_response, list_stones, set_board_size},
    reset::{draw_reset, reset_button_top_left, RESET_BUTTON_SIZE},
    routine::Routine,
};
use gtp::{controller::Engine, Command};
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
    HumanTurn = 1,
    MachineTurn = 2,
}

fn do_machine_move(ctrl: &mut Engine) {
    ctrl.send(Command::new_with_args("genmove", |e| e.s("black")));
    info!("waiting for machine response");
    let resp = get_response(ctrl);
    info!("machine: {}", resp.text());
}

pub struct MachineGame {
    board: Board,
    current_turn: Turn,
}

impl MachineGame {
    pub fn new() -> MachineGame {
        MachineGame {
            board: Board::new(9),
            current_turn: Turn::MachineTurn,
        }
    }

    fn draw_turn(&self, fb: &mut Framebuffer, refresh: bool) {
        let rect_width = 550;
        info!("draw_turn {:?}", self.current_turn);
        let text = if self.current_turn == Turn::HumanTurn {
            "Human turn"
        } else {
            "Machine turn"
        };
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

    fn set_turn(&mut self, turn: Turn, fb: &mut Framebuffer) {
        info!("Set turn {turn:?}");
        self.current_turn = turn;
        self.draw_turn(fb, true);
    }

    fn reset_game(&self, ctrl: &mut Engine, fb: &mut Framebuffer) {
        clear_board(ctrl);
        do_machine_move(ctrl);
        self.redraw_stones(ctrl, fb);
    }

    fn redraw_stones(&self, ctrl: &mut Engine, fb: &mut Framebuffer) {
        let start = Instant::now();
        let white_stones = list_stones(ctrl, "white");
        let black_stones = list_stones(ctrl, "black");
        self.board.draw_board(fb, &white_stones, &black_stones);
        self.draw_turn(fb, false);
        draw_reset(&self.board, fb);
        refresh(fb);
        let elapsed = start.elapsed();
        info!("redraw elapsed: {:.2?}", elapsed);
    }
}

impl Routine for MachineGame {
    fn init(&mut self, fb: &mut Framebuffer, ctrl: &mut Engine) {
        set_board_size(ctrl, self.board.board_size);
        self.reset_game(ctrl, fb);
        self.set_turn(Turn::HumanTurn, fb);
    }

    fn on_multitouch_event(
        &mut self,
        ctx: &mut appctx::ApplicationContext<'_>,
        event: MultitouchEvent,
        ctrl: &mut Engine,
    ) {
        match event {
            MultitouchEvent::Press { finger } => {
                if self.current_turn != Turn::HumanTurn {
                    info!("Ignoring touch, as machine turn");
                    return;
                }
                let fb = ctx.get_framebuffer_ref();

                let rbtl = reset_button_top_left(&self.board);
                if (finger.pos.x as i32) >= rbtl.x
                    && (finger.pos.x as i32) < (rbtl.x + RESET_BUTTON_SIZE.x as i32)
                    && (finger.pos.y as i32) >= rbtl.y
                    && (finger.pos.y as i32) < (rbtl.y + RESET_BUTTON_SIZE.y as i32)
                {
                    *CURRENT_MODE.lock().unwrap() = crate::chooser::Mode::Chooser;
                    ctx.stop();
                    return;
                }

                let point = self.board.nearest_spot(finger.pos.x, finger.pos.y);
                let pos = finger.pos;
                if point.x >= self.board.board_size || point.y >= self.board.board_size {
                    info!("Bad point {point:?}");
                    return;
                }
                info!("Drawing: {point:?} for {pos:?}");
                if !do_human_move(ctrl, point, "white") {
                    info!("Bad human move");
                    return;
                }
                self.set_turn(Turn::MachineTurn, fb);
                self.redraw_stones(ctrl, fb);
                do_machine_move(ctrl);
                self.redraw_stones(ctrl, fb);
                self.set_turn(Turn::HumanTurn, fb);
            }
            _ => {}
        }
    }
}
