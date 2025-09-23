use crate::{
    board::Board,
    chooser::CURRENT_MODE,
    drawing::refresh,
    reset::{draw_reset, reset_button_top_left},
    routine::Routine,
};
use chrono::{DateTime, TimeDelta, Utc};
use core::fmt;
use gtp::{controller::Engine, Entity};
use lazy_static::lazy_static;
use libremarkable::{appctx, framebuffer::core::Framebuffer, input::MultitouchEvent};
use log::{info, warn};
use serde::{de, Deserialize, Serialize};
use sgf_parse::{
    go::{parse, Move, Prop},
    SgfNode,
};
use std::{fs, ops::Deref, sync::Mutex, time::Instant};

const DEFAULT_LOGIN_FILE: &str = "/opt/dragon-go-server-login";
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

fn dragon_date<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct DragonDateStringVisitor;

    impl<'de> de::Visitor<'de> for DragonDateStringVisitor {
        type Value = DateTime<Utc>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string containing a dragongoserver date")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let original_date_str = v.replace("'", "") + "Z";

            match chrono::DateTime::parse_from_rfc3339(&original_date_str) {
                Ok(date) => Ok(date.to_utc()),
                Err(_) => {
                    panic!("bad date: '{}'", original_date_str)
                }
            }
        }
    }

    deserializer.deserialize_any(DragonDateStringVisitor)
}

fn time_remaining<'de, D>(deserializer: D) -> Result<TimeDelta, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct TimeRemainingStringVisitor;

    impl<'de> de::Visitor<'de> for TimeRemainingStringVisitor {
        type Value = TimeDelta;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string containing a time remaining")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v.replace("'", "").chars().next().unwrap() {
                'F' => {
                    // Fischer time
                    let remaining = &v[3..v.find("(").unwrap()];
                    let mut delta = TimeDelta::zero();
                    for piece in remaining.split_whitespace() {
                        let value = match i64::from_str_radix(&piece[..piece.len() - 1], 10) {
                            Ok(v) => v,
                            Err(_) => {
                                panic!("Error parsing '{}' in '{}'", piece, remaining);
                            }
                        };
                        delta += match &piece.chars().last().unwrap() {
                            'd' => TimeDelta::days(value),
                            'h' => TimeDelta::hours(value),
                            default => {
                                panic!("Something else! '{}'", default);
                            }
                        };
                    }
                    Ok(delta)
                }
                default => {
                    panic!("Something else! '{}'", default);
                }
            }
        }
    }

    deserializer.deserialize_any(TimeRemainingStringVisitor)
}

#[derive(Debug, Deserialize)]
struct GameRecord {
    g: String,
    game_id: u32,
    opponent_handle: String,
    player_color: String,
    #[serde(deserialize_with = "dragon_date")]
    lastmove_date: DateTime<Utc>,
    #[serde(deserialize_with = "time_remaining")]
    time_remaining: TimeDelta,
    game_action: u8,
    game_status: String,
    move_id: u32,
    tournament_id: u32,
    shape_id: u32,
    game_type: String,
    game_prio: i32,
    #[serde(deserialize_with = "dragon_date")]
    opponent_lastaccess_date: DateTime<Utc>,
    handicap: u8,
}

// fn draw_status(fb: &mut Framebuffer, text: &str, refresh: bool) {
//     let rect_width = 550;
//     fb.fill_rect(
//         Point2 {
//             x: SPARE_WIDTH as i32,
//             y: 0,
//         },
//         Vector2 {
//             x: rect_width,
//             y: 100,
//         },
//         color::WHITE,
//     );
//     fb.draw_text(
//         Point2 {
//             x: SPARE_WIDTH as f32,
//             y: 100.0,
//         },
//         text,
//         100.0,
//         color::BLACK,
//         false,
//     );

//     if refresh {
//         refresh_with_options(
//             fb,
//             &mxcfb_rect {
//                 top: 0,
//                 left: SPARE_WIDTH as u32,
//                 width: rect_width,
//                 height: 100,
//             },
//             waveform_mode::WAVEFORM_MODE_AUTO,
//         );
//     }
// }

pub struct DragonGoServer {
    client: reqwest::blocking::Client,
    white_stones: Vec<Entity>,
    black_stones: Vec<Entity>,
    board: Option<Board>,
}

impl DragonGoServer {
    fn redraw_stones(&self, fb: &mut Framebuffer) {
        let start = Instant::now();
        if let Some(ref board) = self.board {
            board.draw_board(fb, &self.white_stones, &self.black_stones);
            draw_reset(board, fb);
        }
        refresh(fb);
        let elapsed = start.elapsed();
        info!("redraw elapsed: {:.2?}", elapsed);
    }
}

impl Default for DragonGoServer {
    fn default() -> Self {
        Self {
            white_stones: vec![],
            black_stones: vec![],
            client: reqwest::blocking::ClientBuilder::new()
                .cookie_store(true)
                .build()
                .unwrap(),
            board: None,
        }
    }
}

fn get_sgf_properties_for_node(node: &SgfNode<Prop>) -> Vec<Prop> {
    let mut output = vec![];
    for prop in node.properties() {
        output.push(prop.clone());
    }
    for child in node.children() {
        output.append(&mut get_sgf_properties_for_node(child));
    }
    output
}

fn get_sgf_properties(raw_sgf: &str) -> Vec<Prop> {
    let mut output = vec![];
    for node in parse(&raw_sgf).unwrap() {
        output.append(&mut get_sgf_properties_for_node(&node));
    }
    output
}

impl Routine for DragonGoServer {
    fn init(&mut self, fb: &mut Framebuffer, _ctrl: &mut Engine) {
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
                let mut first_expiring_game: Option<GameRecord> = None;
                // info!("Status: {}", status);
                for record_raw_res in csv::ReaderBuilder::new()
                    .has_headers(false)
                    .flexible(true)
                    .from_reader(status.as_bytes())
                    .records()
                {
                    // info!("Record raw: {:#?}", record_raw_res);
                    let record_raw = record_raw_res.unwrap();
                    if !record_raw.get(0).unwrap().starts_with("G") {
                        continue;
                    }
                    let record: GameRecord = record_raw.deserialize(None).unwrap();
                    info!("Game: {:#?}", record);
                    if first_expiring_game
                        .as_ref()
                        .and_then(|g| Some(g.time_remaining > record.time_remaining))
                        .unwrap_or(true)
                    {
                        first_expiring_game.replace(record);
                    }
                }

                if let Some(game) = first_expiring_game {
                    let raw_sgf = self
                    .client
                    .get(format!(
                        "https://www.dragongoserver.net/sgf.php?gid={}&owned_comments=N&quick_mode=0&no_cache=0",
                        game.game_id
                    ))
                    .send()
                    .unwrap()
                    .text()
                    .unwrap();
                    let props = get_sgf_properties(&raw_sgf);
                    for prop in props {
                        match prop {
                            Prop::W(white_move) => {
                                if let Move::Move(point) = white_move {
                                    self.white_stones.push(Entity::Vertex((
                                        (point.x + 1) as i32,
                                        (point.y + 1) as i32,
                                    )))
                                }
                            }
                            Prop::AB(black_moves) => {
                                for point in black_moves {
                                    self.black_stones.push(Entity::Vertex((
                                        (point.x + 1) as i32,
                                        (point.y + 1) as i32,
                                    )))
                                }
                            }
                            Prop::SZ(size) => {
                                self.board = Some(Board::new(size.0));
                            }
                            other => {
                                info!("Other prop: {other}")
                            }
                        }
                    }
                }
            }
        }
        *LOGIN_INFO.lock().expect("Can lock login_info") = login_info;
        self.redraw_stones(fb);
    }

    fn on_multitouch_event(
        &mut self,
        ctx: &mut appctx::ApplicationContext<'_>,
        event: MultitouchEvent,
        _ctrl: &mut Engine,
    ) {
        match event {
            MultitouchEvent::Press { finger } => {
                let start = Instant::now();
                let fb = ctx.get_framebuffer_ref();

                if let Some(ref board) = self.board {
                    let rbtl = reset_button_top_left(board);
                    if (finger.pos.x as i32) >= rbtl.x
                        && (finger.pos.x as i32) < (rbtl.x + rbtl.x as i32)
                        && (finger.pos.y as i32) >= rbtl.y
                        && (finger.pos.y as i32) < (rbtl.y + rbtl.y as i32)
                    {
                        *CURRENT_MODE.lock().unwrap() = crate::chooser::Mode::Chooser;
                        ctx.stop();
                        return;
                    }

                    let point = board.nearest_spot(finger.pos.x, finger.pos.y);
                    let pos = finger.pos;
                    if point.x >= board.board_size || point.y >= board.board_size {
                        info!("Bad point {point:?}");
                        return;
                    }
                    info!("Drawing: {point:?} for {pos:?}");
                    board.refresh_and_draw_one_piece(fb, point.x, point.y, true);
                }

                let elapsed = start.elapsed();
                info!("touch elapsed: {:.2?}", elapsed);
            }
            _ => {}
        }
    }
}
