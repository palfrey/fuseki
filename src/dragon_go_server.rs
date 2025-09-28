use crate::{
    board::{Board, AVAILABLE_WIDTH},
    chooser::CURRENT_MODE,
    drawing::{draw_button, refresh, refresh_with_options},
    reset::{draw_reset, reset_button_top_left, RESET_BUTTON_SIZE},
    routine::Routine,
};
use chrono::{DateTime, TimeDelta, Utc};
use core::fmt;
use gtp::controller::Engine;
use lazy_static::lazy_static;
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
use log::{error, info, warn};
use serde::{de, Deserialize, Serialize};
use sgf_parse::{
    go::{parse, Move, Prop},
    SgfNode,
};
use std::{
    fs,
    ops::Deref,
    sync::Mutex,
    time::{Duration, Instant},
};

const DEFAULT_LOGIN_FILE: &str = "/opt/dragon-go-server-login";
lazy_static! {
    static ref LOGIN_FILE: Mutex<String> = Mutex::new(DEFAULT_LOGIN_FILE.to_string());
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
struct LoginInfo {
    username: String,
    password: String,
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

#[derive(Debug, Deserialize, PartialEq, Clone)]
enum PlayerColor {
    #[serde(alias = "B")]
    Black,
    #[serde(alias = "W")]
    White,
}

#[derive(Debug, Deserialize)]
struct GameRecord {
    g: String,
    game_id: u32,
    opponent_handle: String,
    player_color: PlayerColor,
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

pub const UNDO_BUTTON_SIZE: Vector2<u32> = Vector2 { x: 250, y: 95 };
pub const COMMIT_BUTTON_SIZE: Vector2<u32> = Vector2 { x: 350, y: 95 };

pub struct BoardConfig {
    board: Board,
    undo_button_top_left: Point2<i32>,
    commit_button_top_left: Point2<i32>,
    player_color: PlayerColor,
    game_id: u32,
    last_move_id: u32,
    opponent_handle: String,
}

pub struct DragonGoServer {
    client: reqwest::blocking::Client,
    white_stones: Vec<Point2<u8>>,
    black_stones: Vec<Point2<u8>>,
    board_config: Option<BoardConfig>,
    chosen: Option<Point2<u8>>,
    login_info: LoginInfo,
    fb: Option<&'static mut Framebuffer>,
}

enum Actions {
    Refresh,
    Exit,
}

struct Button {
    text: String,
    top_left: Point2<i32>,
    size: Vector2<u32>,
    action: Actions,
}

const BUTTON_WIDTH: u32 = 700;
const TOP_LEFT_X: i32 =
    ((libremarkable::dimensions::DISPLAYWIDTH as u32 - BUTTON_WIDTH) / 2) as i32;

lazy_static! {
    static ref NO_GAME_BUTTONS: Vec<Button> = {
        vec![
            Button {
                text: "Refresh".to_string(),
                top_left: Point2 {
                    x: TOP_LEFT_X,
                    y: 100,
                },
                size: Vector2 {
                    x: BUTTON_WIDTH,
                    y: 95,
                },
                action: Actions::Refresh,
            },
            Button {
                text: "Exit".to_string(),
                top_left: Point2 {
                    x: TOP_LEFT_X,
                    y: 300,
                },
                size: Vector2 {
                    x: BUTTON_WIDTH,
                    y: 95,
                },
                action: Actions::Exit,
            },
        ]
    };
}

impl DragonGoServer {
    pub fn new() -> Self {
        Self {
            white_stones: vec![],
            black_stones: vec![],
            client: reqwest::blocking::ClientBuilder::new()
                .cookie_store(true)
                .build()
                .unwrap(),
            board_config: None,
            chosen: None,
            login_info: LoginInfo::default(),
            fb: None,
        }
    }

    fn draw_choices(&self, fb: &mut Framebuffer) {
        if let Some(ref board_config) = self.board_config {
            if let Some(ref chosen) = self.chosen {
                draw_button(
                    fb,
                    "Undo",
                    board_config.undo_button_top_left,
                    UNDO_BUTTON_SIZE,
                );
                draw_button(
                    fb,
                    "Commit",
                    board_config.commit_button_top_left,
                    COMMIT_BUTTON_SIZE,
                );

                board_config.board.draw_piece(
                    fb,
                    chosen.x,
                    chosen.y,
                    board_config.player_color == PlayerColor::White,
                );
            }
        }
    }

    fn redraw_stones(&self, fb: &mut Framebuffer) {
        fb.clear();
        let start = Instant::now();
        if let Some(ref board_config) = self.board_config {
            board_config
                .board
                .draw_board(fb, &self.white_stones, &self.black_stones);
            draw_reset(&board_config.board, fb);
            self.draw_choices(fb);
            self.draw_status(fb, &format!("opp: {}", &board_config.opponent_handle), false, 0);
            self.draw_status(fb, &format!("colour: {:?}",board_config.player_color), false, 120);
        } else {
            for button in NO_GAME_BUTTONS.iter() {
                draw_button(fb, &button.text, button.top_left, button.size);
            }
        }
        refresh(fb);
        let elapsed = start.elapsed();
        info!("redraw elapsed: {:.2?}", elapsed);
    }

    fn load_next_game(&mut self) {
        self.white_stones.clear();
        self.black_stones.clear();
        let login_resp = self
            .client
            .post(format!(
                "https://www.dragongoserver.net/login.php?quick_mode=1&userid={}&passwd={}",
                self.login_info.username, self.login_info.password
            ))
            .send()
            .unwrap();
        // info!("Headers: {:#?}", &login_resp.headers());
        let login_text = login_resp.text().unwrap();
        if !login_text.contains("Ok") {
            warn!("Error logging in: {}", login_text);
            return;
        }
        let status = self
            .client
            .get(format!(
                "https://www.dragongoserver.net/quick_status.php?user={}&version=2",
                self.login_info.username
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
                            self.white_stones.push(Point2 {
                                x: (point.x + 1),
                                y: (point.y + 1),
                            })
                        }
                    }
                    Prop::B(black_move) => {
                        if let Move::Move(point) = black_move {
                            self.black_stones.push(Point2 {
                                x: (point.x + 1),
                                y: (point.y + 1),
                            })
                        }
                    }
                    Prop::AB(black_moves) => {
                        for point in black_moves {
                            self.black_stones.push(Point2 {
                                x: (point.x + 1),
                                y: (point.y + 1),
                            })
                        }
                    }
                    Prop::SZ(size) => {
                        let board = Board::new(size.0);
                        let undo_button_top_left = Point2 {
                            x: (board.spare_width + AVAILABLE_WIDTH / 2 - 170) as i32,
                            y: 20,
                        };
                        let commit_button_top_left = Point2 {
                            x: (board.spare_width + AVAILABLE_WIDTH / 2 - 640) as i32,
                            y: 20,
                        };
                        self.board_config = Some(BoardConfig {
                            player_color: game.player_color.clone(),
                            board,
                            undo_button_top_left,
                            commit_button_top_left,
                            game_id: game.game_id,
                            last_move_id: game.move_id,
                            opponent_handle: game
                                .opponent_handle
                                .strip_prefix("'")
                                .unwrap()
                                .strip_suffix("'")
                                .unwrap()
                                .to_string(),
                        });
                    }
                    other => {
                        info!("Other prop: {other}")
                    }
                }
            }
        }
    }

    fn draw_status(&self, fb: &mut Framebuffer, text: &str, refresh: bool, offset: u16) {
        if let Some(ref board_config) = self.board_config {
            let rect_width = 550;
            fb.fill_rect(
                Point2 {
                    x: board_config.board.spare_width as i32,
                    y: offset as i32,
                },
                Vector2 {
                    x: rect_width,
                    y: (80 + offset) as u32,
                },
                color::WHITE,
            );
            fb.draw_text(
                Point2 {
                    x: board_config.board.spare_width as f32,
                    y: (100 + offset) as f32,
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
                        top: offset as u32,
                        left: board_config.board.spare_width as u32,
                        width: rect_width,
                        height: 80,
                    },
                    waveform_mode::WAVEFORM_MODE_AUTO,
                );
            }
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
    fn init(&mut self, fb: &'static mut Framebuffer, _ctrl: &mut Engine) {
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
        }
        self.login_info = login_info;
        self.fb = Some(fb);
    }

    fn update_loop(&mut self) -> Option<Duration> {
        info!("Update game");
        if self.chosen.is_none() {
            self.load_next_game();
            let current_fb = self.fb.take();
            if current_fb.is_some() {
                let mut fb = current_fb.unwrap();
                self.redraw_stones(&mut fb);
                let _empty = self.fb.insert(fb);
            } else {
                error!("No framebuffer!");
            }
        } else {
            info!("Chosen set, not updating");
        }
        Some(Duration::from_secs(30))
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

                if let Some(ref board_config) = self.board_config {
                    let board = &board_config.board;
                    let rbtl = reset_button_top_left(board);
                    info!("rbtl: {rbtl:?}");
                    if (finger.pos.x as i32) >= rbtl.x
                        && (finger.pos.x as i32) < (rbtl.x + RESET_BUTTON_SIZE.x as i32)
                        && (finger.pos.y as i32) >= rbtl.y
                        && (finger.pos.y as i32) < (rbtl.y + RESET_BUTTON_SIZE.y as i32)
                    {
                        *CURRENT_MODE.lock().unwrap() = crate::chooser::Mode::Chooser;
                        ctx.stop();
                        return;
                    }

                    if self.chosen.is_none() {
                        let point = board.nearest_spot(finger.pos.x, finger.pos.y);
                        if point.x >= board.board_size || point.y >= board.board_size {
                            info!("Bad point {point:?} from {:?}", finger.pos);
                            return;
                        }
                        // FIXME: Because GTP points are offset
                        let offset_point = Point2 {
                            x: point.x + 1,
                            y: point.y + 1,
                        };
                        if self.white_stones.contains(&offset_point)
                            || self.black_stones.contains(&offset_point)
                        {
                            info!("Can't use existing point");
                            return;
                        }
                        info!("Drawing: {point:?} for {:?}", finger.pos);
                        self.chosen = Some(point);
                        self.redraw_stones(fb);
                    } else {
                        if (finger.pos.x as i32) >= board_config.undo_button_top_left.x
                            && (finger.pos.x as i32)
                                < (board_config.undo_button_top_left.x + UNDO_BUTTON_SIZE.x as i32)
                            && (finger.pos.y as i32) >= board_config.undo_button_top_left.y
                            && (finger.pos.y as i32)
                                < (board_config.undo_button_top_left.y + UNDO_BUTTON_SIZE.y as i32)
                        {
                            self.chosen = None;
                            self.redraw_stones(fb);
                        }

                        if (finger.pos.x as i32) >= board_config.commit_button_top_left.x
                            && (finger.pos.x as i32)
                                < (board_config.commit_button_top_left.x
                                    + COMMIT_BUTTON_SIZE.x as i32)
                            && (finger.pos.y as i32) >= board_config.commit_button_top_left.y
                            && (finger.pos.y as i32)
                                < (board_config.commit_button_top_left.y
                                    + COMMIT_BUTTON_SIZE.y as i32)
                        {
                            let chosen = self.chosen.take().unwrap();
                            // because i
                            let column_chars = (0..board.board_size + 1)
                                .into_iter()
                                .map(|x| char::from_u32(('a' as u32) + x as u32).unwrap())
                                .filter(|p| *p != 'i')
                                .collect::<Vec<char>>();
                            let url = format!(
                                "https://www.dragongoserver.net/quick_do.php?obj=game&cmd=move&gid={}&move_id={}&move={}{}",
                                board_config.game_id,
                                board_config.last_move_id,
                                column_chars.get(chosen.x as usize).unwrap(),
                                board_config.board.board_size-chosen.y
                            );
                            info!("Url: {url}");
                            let move_resp = self.client.post(url).send().unwrap().text().unwrap();
                            info!("Move resp: {}", move_resp);

                            self.load_next_game();
                            self.redraw_stones(fb);
                        }
                    }
                } else {
                    for button in NO_GAME_BUTTONS.iter() {
                        if (finger.pos.x as i32) >= button.top_left.x
                            && (finger.pos.x as i32) < (button.top_left.x + button.size.x as i32)
                            && (finger.pos.y as i32) >= button.top_left.y
                            && (finger.pos.y as i32) < (button.top_left.y + button.size.y as i32)
                        {
                            match button.action {
                                Actions::Refresh => {
                                    self.load_next_game();
                                    self.redraw_stones(fb);
                                }
                                Actions::Exit => {
                                    *CURRENT_MODE.lock().unwrap() = crate::chooser::Mode::Chooser;
                                    ctx.stop();
                                    return;
                                }
                            }
                        }
                    }
                }

                let elapsed = start.elapsed();
                info!("touch elapsed: {:.2?}", elapsed);
            }
            _ => {}
        }
    }
}
