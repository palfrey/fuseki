use std::sync::Mutex;
use std::time::Duration;

use ::gtp::{controller::Engine, Command};
use libremarkable::appctx;
use log::info;

use crate::board::BOARD_SIZE;

mod board;
mod drawing;
mod gtp;
mod machine_game;
mod reset;

#[derive(PartialEq, Debug, Clone, Copy)]
enum Mode {
    Chooser = 1,
    AgainstMachine = 2,
    Atari = 3,
}

static CURRENT_MODE: Mutex<Mode> = Mutex::new(Mode::Chooser);

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
    info!("Init complete. Beginning event dispatch...");

    machine_game::run_game(&mut ctrl, fb, &mut app);
}
