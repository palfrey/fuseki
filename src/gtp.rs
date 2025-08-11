use std::time::{Duration, Instant};

use gtp::{controller::Engine, Command, Response};
use libremarkable::cgmath::Point2;
use log::info;

pub fn list_stones(ctrl: &mut Engine, color: &str) -> Vec<gtp::Entity> {
    let start = Instant::now();
    let cmd = Command::new_with_args("list_stones", |e| e.s(color));
    // info!("list_stones: {}", cmd.to_string());
    ctrl.send(cmd);
    let resp = get_response(ctrl);
    // info!("list_stones resp: {}", resp.text());
    let ev = resp.entities(|ep| {
        let mut ret = ep;
        while !ret.is_eof() {
            ret = ret.vertex();
        }
        ret
    });
    let elapsed = start.elapsed();
    info!("list_stones elapsed: {:.2?}", elapsed);
    return ev.unwrap();
}

pub fn get_response(ctrl: &mut Engine) -> Response {
    loop {
        match ctrl.wait_response(Duration::from_secs(1)) {
            Ok(resp) => {
                return resp;
            }
            Err(gtp::controller::Error::PollAgain) => continue,
            Err(err) => {
                panic!("Other error {err:?}")
            }
        }
    }
}

pub fn do_human_move(ctrl: &mut Engine, pos: Point2<u8>) -> bool {
    let cmd = Command::new_with_args("play", |e| {
        e.s("white")
            .v(((pos.x + 1) as i32, (pos.y + 1) as i32))
            .list()
    });
    info!("human: {}", cmd.to_string());
    ctrl.send(cmd);
    let resp = get_response(ctrl);
    info!("human resp: {}", resp.text());
    return resp.text() == "";
}

pub fn clear_board(ctrl: &mut Engine) {
    ctrl.send(Command::new_with_args("clear_board", |e| e));
    let resp = get_response(ctrl);
    info!("clear_board: {}", resp.text());
}
