use ::gtp::controller::Engine;
use libremarkable::{appctx, input::InputEvent};
use log::info;

use crate::{
    chooser::{Mode, CURRENT_MODE},
    routine::Routine,
};

mod atari_game;
mod board;
mod chooser;
mod dragon_go_server;
mod drawing;
mod gtp;
mod machine_game;
mod reset;
mod routine;

fn main() {
    env_logger::init();
    let mut app: appctx::ApplicationContext<'_> = appctx::ApplicationContext::default();

    let fb = app.get_framebuffer_ref();

    info!("Starting GnuGo");
    let gnugo_path = std::env::var("GNUGO_BINARY").unwrap_or("/home/root/gnugo".into());
    let mut ctrl = Engine::new(&gnugo_path, &["--mode", "gtp", "--level", "8"]);
    ctrl.start().expect("Failure to launch gnugo");
    info!("Init complete. Beginning event dispatch...");

    let mut previous_mode: Option<Mode> = None;

    loop {
        info!("Starting mode loop");
        let current_mode = *CURRENT_MODE.lock().expect("Working lock");
        let mut current_routine: Box<dyn Routine> = match current_mode {
            Mode::Chooser => Box::new(chooser::Chooser {}),
            Mode::AgainstMachine => Box::new(machine_game::MachineGame::new()),
            Mode::Atari => Box::new(atari_game::AtariGame::new()),
            Mode::DragonGoServer => Box::new(dragon_go_server::DragonGoServer::new()),
            Mode::Exit => {
                break;
            }
        };
        if previous_mode.is_none() || current_mode != previous_mode.unwrap_or(Mode::Chooser) {
            info!("New mode: {current_mode:?}");
            current_routine.init(fb, &mut ctrl);
        }
        previous_mode = Some(current_mode);
        app.start_event_loop(true, true, true, |ctx, evt| match evt {
            InputEvent::MultitouchEvent { event } => {
                current_routine.on_multitouch_event(ctx, event, &mut ctrl);
            }
            ev => {
                info!("event: {ev:?}");
            }
        });
    }
}
