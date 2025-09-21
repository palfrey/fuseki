use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{fs, ops::Deref, sync::Mutex, time::Instant};

use gtp::controller::Engine;
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
use log::{info, warn};

use crate::{
    board::{
        draw_board, nearest_spot, refresh_and_draw_one_piece, AVAILABLE_WIDTH, BOARD_SIZE,
        SPARE_WIDTH,
    },
    chooser::CURRENT_MODE,
    drawing::{draw_text, refresh, refresh_with_options},
    reset::{draw_reset, RESET_BUTTON_SIZE, RESET_BUTTON_TOP_LEFT},
    routine::Routine,
};

const DEFAULT_LOGIN_FILE: &str = "/tmp/dragon-go-server-login";
lazy_static! {
    static ref LOGIN_FILE: Mutex<String> = Mutex::new(DEFAULT_LOGIN_FILE.to_string());
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
struct LoginInfo {
    username: String,
    password: String,
}

lazy_static! {
    static ref LOGIN_INFO: Mutex<LoginInfo> = Mutex::new(LoginInfo::default());
}

#[derive(Debug, Deserialize)]
struct GameRecord {
    g: String,
    game_id: String,
    opponent_handle: String,
    player_color: String,
    lastmove_date: String,
    time_remaining: String,
    game_action: u8,
    game_status: String,
    move_id: u32,
    tournament_id: u32,
    shape_id: u32,
    game_type: String,
    game_prio: i32,
    opponent_lastaccess_date: String,
    handicap: u8,
}

fn draw_status(fb: &mut Framebuffer, text: &str, refresh: bool) {
    let rect_width = 550;
    fb.fill_rect(
        Point2 {
            x: SPARE_WIDTH as i32,
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
            x: SPARE_WIDTH as f32,
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
                left: SPARE_WIDTH as u32,
                width: rect_width,
                height: 100,
            },
            waveform_mode::WAVEFORM_MODE_AUTO,
        );
    }
}

fn redraw_stones(ctrl: &mut Engine, fb: &mut Framebuffer) {
    let start = Instant::now();
    // let white_stones = list_stones(ctrl, "white");
    // let black_stones = list_stones(ctrl, "black");
    // draw_board(fb, white_stones, black_stones);
    draw_reset(fb);
    refresh(fb);
    let elapsed = start.elapsed();
    info!("redraw elapsed: {:.2?}", elapsed);
}

fn on_multitouch_event(
    ctx: &mut appctx::ApplicationContext<'_>,
    event: MultitouchEvent,
    ctrl: &mut Engine,
) {
    match event {
        MultitouchEvent::Press { finger } => {
            let start = Instant::now();
            let fb = ctx.get_framebuffer_ref();

            if (finger.pos.x as i32) >= RESET_BUTTON_TOP_LEFT.x
                && (finger.pos.x as i32) < (RESET_BUTTON_TOP_LEFT.x + RESET_BUTTON_SIZE.x as i32)
                && (finger.pos.y as i32) >= RESET_BUTTON_TOP_LEFT.y
                && (finger.pos.y as i32) < (RESET_BUTTON_TOP_LEFT.y + RESET_BUTTON_SIZE.y as i32)
            {
                *CURRENT_MODE.lock().unwrap() = crate::chooser::Mode::Chooser;
                ctx.stop();
                return;
            }

            let point = nearest_spot(finger.pos.x, finger.pos.y);
            let pos = finger.pos;
            if point.x >= BOARD_SIZE || point.y >= BOARD_SIZE {
                info!("Bad point {point:?}");
                return;
            }
            info!("Drawing: {point:?} for {pos:?}");
            refresh_and_draw_one_piece(fb, point.x, point.y, true);

            let elapsed = start.elapsed();
            info!("touch elapsed: {:.2?}", elapsed);
        }
        _ => {}
    }
}

pub struct DragonGoServer {
    client: reqwest::blocking::Client,
}

impl Default for DragonGoServer {
    fn default() -> Self {
        Self {
            client: reqwest::blocking::ClientBuilder::new()
                .cookie_store(true)
                .build()
                .unwrap(),
        }
    }
}

impl Routine for DragonGoServer {
    fn init(&self, fb: &mut Framebuffer, ctrl: &mut Engine) {
        let current_login_file = LOGIN_FILE.lock().expect("get login_file");
        let login_raw = fs::read(current_login_file.deref());
        let login_info: LoginInfo = match login_raw {
            Ok(raw) => match serde_json::from_slice(&raw) {
                Ok(li) => li,
                Err(err) => {
                    warn!(
                        "Error loading login data from {}: {}",
                        current_login_file, err
                    );
                    LoginInfo::default()
                }
            },
            Err(err) => {
                warn!("Can't read login data from {}: {}", current_login_file, err);
                LoginInfo::default()
            }
        };
        if login_info == LoginInfo::default() {
            let dumped = serde_json::to_vec_pretty(&login_info).expect("can dump login info");
            fs::write(current_login_file.deref(), dumped).expect("Can write login info");
            info!("Dumped default login file");
        } else {
            info!("Loaded login info");
            let login_resp = self
                .client
                .post(format!(
                    "https://www.dragongoserver.net/login.php?quick_mode=1&userid={}&passwd={}",
                    login_info.username, login_info.password
                ))
                .send()
                .unwrap();
            // info!("Headers: {:#?}", &login_resp.headers());
            let login_text = login_resp.text().unwrap();
            if !login_text.contains("Ok") {
                warn!("Error logging in: {}", login_text);
            } else {
                let status = self
                    .client
                    .get(format!(
                        "https://www.dragongoserver.net/quick_status.php?user={}&version=2",
                        login_info.username
                    ))
                    .send()
                    .unwrap()
                    .text()
                    .unwrap();
                info!("Status: {}", status);
                for record_raw_res in csv::ReaderBuilder::new()
                    .has_headers(false)
                    .flexible(true)
                    .from_reader(status.as_bytes())
                    .records()
                {
                    info!("Record raw: {:#?}", record_raw_res);
                    let record_raw = record_raw_res.unwrap();
                    if !record_raw.get(0).unwrap().starts_with("G") {
                        continue;
                    }
                    let record: GameRecord = record_raw.deserialize(None).unwrap();
                    info!("Game: {:#?}", record);
                }
            }
        }
        *LOGIN_INFO.lock().expect("Can lock login_info") = login_info;
    }

    fn on_multitouch_event(
        &self,
        ctx: &mut appctx::ApplicationContext<'_>,
        event: MultitouchEvent,
        ctrl: &mut Engine,
    ) {
        on_multitouch_event(ctx, event, ctrl);
    }
}
