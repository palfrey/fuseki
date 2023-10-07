use libremarkable::{
    appctx, cgmath,
    framebuffer::{
        common::{color, display_temp, dither_mode, mxcfb_rect, waveform_mode},
        core::Framebuffer,
        FramebufferDraw, FramebufferRefresh,
    },
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

fn loop_update_topbar(app: &mut appctx::ApplicationContext<'_>, millis: u64) {}

fn draw_piece(fb: &mut Framebuffer, x: u8, y: u8, white: bool) {
    let point = cgmath::Point2 {
        x: (SPARE_WIDTH + (SQUARE_SIZE * x as u16)) as i32,
        y: (SPARE_HEIGHT + (SQUARE_SIZE * y as u16)) as i32,
    };
    fb.fill_circle(point, CIRCLE_RADIUS as u32, color::BLACK);
    if white {
        fb.fill_circle(point, (CIRCLE_RADIUS - CIRCLE_BORDER) as u32, color::WHITE);
    }
}

fn main() {
    env_logger::init();
    let mut app: appctx::ApplicationContext<'_> = appctx::ApplicationContext::default();
    app.clear(true);

    let fb = app.get_framebuffer_ref();

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
    draw_piece(fb, 0, 0, false);
    draw_piece(fb, 1, 1, true);
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

    // Get a &mut to the framebuffer object, exposing many convenience functions
    let appref = app.upgrade_ref();
    let clock_thread = std::thread::spawn(move || {
        loop_update_topbar(appref, 30 * 1000);
    });

    info!("Init complete. Beginning event dispatch...");

    // Blocking call to process events from digitizer + touchscreen + physical buttons
    app.start_event_loop(true, true, true, |ctx, evt| match evt {
        // InputEvent::WacomEvent { event } => on_wacom_input(ctx, event),
        // InputEvent::MultitouchEvent { event } => on_touch_handler(ctx, event),
        // InputEvent::GPIO { event } => on_button_press(ctx, event),
        _ => {}
    });
    clock_thread.join().unwrap();
}
