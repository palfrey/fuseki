use gtp::controller::Engine;
use libremarkable::{appctx, framebuffer::core::Framebuffer, input::MultitouchEvent};

pub trait Routine {
    fn init(&mut self, fb: &mut Framebuffer, ctrl: &mut Engine);
    fn on_multitouch_event(
        &mut self,
        ctx: &mut appctx::ApplicationContext<'_>,
        event: MultitouchEvent,
        ctrl: &mut Engine,
    );
}
