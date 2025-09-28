use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{park_timeout, spawn, JoinHandle},
};

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

    info!("Starting GnuGo");
    let gnugo_path = std::env::var("GNUGO_BINARY").unwrap_or("/home/root/gnugo".into());
    let mut ctrl = Engine::new(&gnugo_path, &["--mode", "gtp", "--level", "8"]);
    ctrl.start().expect("Failure to launch gnugo");
    info!("Init complete. Beginning event dispatch...");

    let mut previous_mode: Option<Mode> = None;
    let thread_running = Arc::new(AtomicBool::new(false));

    let mut current_thread: Option<JoinHandle<_>> = None;

    loop {
        info!("Starting mode loop");
        let current_mode = *CURRENT_MODE.lock().expect("Working lock");
        let current_routine: Box<dyn Routine> = match current_mode {
            Mode::Chooser => Box::new(chooser::Chooser {}),
            Mode::AgainstMachine => Box::new(machine_game::MachineGame::new()),
            Mode::Atari => Box::new(atari_game::AtariGame::new()),
            Mode::DragonGoServer => Box::new(dragon_go_server::DragonGoServer::new()),
            Mode::Exit => {
                break;
            }
        };
        let arc_routine = Arc::new(Mutex::new(current_routine));
        let local_routine = arc_routine.clone();
        if previous_mode.is_none() || current_mode != previous_mode.unwrap_or(Mode::Chooser) {
            info!("New mode: {current_mode:?}");
            let fb = app.get_framebuffer_ref();
            arc_routine
                .lock()
                .expect("Can get routine")
                .init(fb, &mut ctrl);
            if let Some(old_thread) = current_thread {
                thread_running.store(false, Ordering::Relaxed);
                old_thread.thread().unpark();
                old_thread.join().unwrap();
            }
            thread_running.store(true, Ordering::Relaxed);
            let local_thread_running = thread_running.clone();
            current_thread = Some(spawn(move || loop {
                if !local_thread_running.load(Ordering::Relaxed) {
                    info!("End of thread");
                    break;
                }
                let how_long = local_routine.lock().expect("can unlock").update_loop();
                match how_long {
                    Some(to_wait) => {
                        park_timeout(to_wait);
                    }
                    None => {
                        break;
                    }
                }
            }));
        }
        previous_mode = Some(current_mode);
        info!("start event loop");
        app.start_event_loop(false, true, false, |ctx, evt| match evt {
            InputEvent::MultitouchEvent { event } => {
                arc_routine
                    .lock()
                    .expect("Get routine")
                    .on_multitouch_event(ctx, event, &mut ctrl);
            }
            ev => {
                info!("event: {ev:?}");
            }
        });
    }
}
