use std::time::Duration;

use gtp::controller::Engine;
use libremarkable::{appctx, framebuffer::core::Framebuffer, input::MultitouchEvent};

pub trait Routine: Send {
    fn init(&mut self, fb: &'static mut Framebuffer, ctrl: &mut Engine);
    fn on_multitouch_event(
        &mut self,
        ctx: &mut appctx::ApplicationContext<'_>,
        event: MultitouchEvent,
        ctrl: &mut Engine,
    );

    fn update_loop(&mut self) -> Option<Duration> {
        None
    }
}
