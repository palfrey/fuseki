use libremarkable::{
    cgmath::{Point2, Vector2},
    framebuffer::{
        common::{color, display_temp, dither_mode, mxcfb_rect, waveform_mode},
        core::Framebuffer,
        FramebufferDraw, FramebufferRefresh,
    },
};

pub fn refresh_with_options(fb: &mut Framebuffer, region: &mxcfb_rect, waveform: waveform_mode) {
    let marker = fb.partial_refresh(
        region,
        libremarkable::framebuffer::PartialRefreshMode::Async,
        waveform,
        display_temp::TEMP_USE_REMARKABLE_DRAW,
        dither_mode::EPDC_FLAG_EXP1,
        0,
        false,
    );
    fb.wait_refresh_complete(marker);
}

pub fn refresh(fb: &mut Framebuffer) {
    refresh_with_options(
        fb,
        &mxcfb_rect {
            top: 0,
            left: 0,
            width: libremarkable::dimensions::DISPLAYWIDTH as u32,
            height: libremarkable::dimensions::DISPLAYHEIGHT as u32,
        },
        waveform_mode::WAVEFORM_MODE_AUTO,
    );
}

pub fn draw_text(fb: &mut Framebuffer, text: &str, top_left: Point2<i32>, size: Vector2<u32>) {
    fb.draw_rect(top_left, size, 5, color::BLACK);
    fb.draw_text(
        Point2 {
            x: (top_left.x + 5) as f32,
            y: (top_left.y + 80) as f32,
        },
        text,
        100.0,
        color::BLACK,
        false,
    );
}
