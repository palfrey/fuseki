use libremarkable::framebuffer::{
    common::{display_temp, dither_mode, mxcfb_rect, waveform_mode},
    core::Framebuffer,
    FramebufferRefresh,
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
