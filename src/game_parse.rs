use libremarkable::cgmath::Point2;
use log::info;
use sgf_parse::{
    go::{parse, Move, Prop},
    SgfNode,
};

#[derive(PartialEq, Debug)]
pub struct GameData {
    pub white_stones: Vec<Point2<u8>>,
    pub black_stones: Vec<Point2<u8>>,
    pub size: u8,
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum GridPoint {
    White,
    Black,
    Empty,
}

fn find_dead_stones(
    grid: &mut [&mut [GridPoint]],
    unknown_spots: Vec<Point2<u8>>,
    size: u8,
) -> Vec<Point2<u8>> {
    // for x in 0..(size as usize) {
    //     for y in 0..(size as usize) {
    //         match grid[x][y] {
    //             GridPoint::White => print!("W"),
    //             GridPoint::Black => print!("B"),
    //             GridPoint::Empty => print!("."),
    //         }
    //     }
    //     println!("");
    // }

    let mut safe_spots = vec![];
    loop {
        let mut new_safe_spot = false;
        for check in &unknown_spots {
            if safe_spots.contains(&check) {
                continue;
            }
            let mut neighbours = vec![];
            if check.x > 0 {
                neighbours.push((check.y, check.x - 1));
            }
            if check.x < (size - 1) {
                neighbours.push((check.y, check.x + 1));
            }
            if check.y > 0 {
                neighbours.push((check.y - 1, check.x));
            }
            if (check.y) < (size - 1) {
                neighbours.push((check.y + 1, check.x));
            }
            if neighbours.iter().any(|(y, x)| {
                grid[*y as usize][*x as usize] == GridPoint::Empty
                    || safe_spots.contains(&&Point2 { x: *x, y: *y })
            }) {
                new_safe_spot = true;
                safe_spots.push(check);
                // println!("New safe {check:?}. Neighbours: {neighbours:?}");
            }
        }
        if !new_safe_spot {
            break;
        }
    }
    let mut dead_stones = vec![];
    for check in &unknown_spots {
        if safe_spots.contains(&check) {
            continue;
        }
        dead_stones.push(check.clone());
    }
    return dead_stones;
}

pub fn get_game_data(raw_sgf: &str) -> GameData {
    let mut gd = GameData {
        white_stones: vec![],
        black_stones: vec![],
        size: 0,
    };
    let props = get_sgf_properties(raw_sgf);

    for prop in &props {
        match prop {
            Prop::SZ(size) => {
                gd.size = size.0;
            }
            _ => {}
        }
    }

    // From https://stackoverflow.com/a/36376568
    let mut grid_raw = vec![GridPoint::Empty; (gd.size * gd.size) as usize];
    let mut grid_base: Vec<_> = grid_raw
        .as_mut_slice()
        .chunks_mut(gd.size as usize)
        .collect();
    let grid = grid_base.as_mut_slice();

    for prop in props {
        let mut current_move = GridPoint::Empty;
        match prop {
            Prop::W(white_move) => {
                if let Move::Move(point) = white_move {
                    gd.white_stones.push(Point2 {
                        x: point.x,
                        y: point.y,
                    });
                    grid[point.y as usize][point.x as usize] = GridPoint::White;
                    current_move = GridPoint::White;
                }
            }
            Prop::B(black_move) => {
                if let Move::Move(point) = black_move {
                    gd.black_stones.push(Point2 {
                        x: point.x,
                        y: point.y,
                    });
                    grid[point.y as usize][point.x as usize] = GridPoint::Black;
                    current_move = GridPoint::Black;
                }
            }
            Prop::AB(black_moves) => {
                for point in black_moves {
                    gd.black_stones.push(Point2 {
                        x: point.x,
                        y: point.y,
                    });
                    grid[point.y as usize][point.x as usize] = GridPoint::Black;
                }
            }
            Prop::AW(white_moves) => {
                for point in white_moves {
                    gd.white_stones.push(Point2 {
                        x: (point.x + 1),
                        y: (point.y + 1),
                    });
                    grid[point.y as usize][point.x as usize] = GridPoint::White;
                }
            }
            other => {
                info!("Other prop: {other}")
            }
        }

        match current_move {
            GridPoint::Empty => {}
            GridPoint::Black => {
                let dead_black_stones = find_dead_stones(grid, gd.black_stones.clone(), gd.size);
                if dead_black_stones.len() > 0 {
                    gd.black_stones = gd
                        .black_stones
                        .iter()
                        .filter(|s| !dead_black_stones.contains(s))
                        .cloned()
                        .collect();
                }
                let dead_white_stones = find_dead_stones(grid, gd.white_stones.clone(), gd.size);
                if dead_white_stones.len() > 0 {
                    gd.white_stones = gd
                        .white_stones
                        .iter()
                        .filter(|s| !dead_white_stones.contains(s))
                        .cloned()
                        .collect();
                }
            }
            GridPoint::White => {
                let dead_white_stones = find_dead_stones(grid, gd.white_stones.clone(), gd.size);
                if dead_white_stones.len() > 0 {
                    gd.white_stones = gd
                        .white_stones
                        .iter()
                        .filter(|s| !dead_white_stones.contains(s))
                        .cloned()
                        .collect();
                }
                let dead_black_stones = find_dead_stones(grid, gd.black_stones.clone(), gd.size);
                if dead_black_stones.len() > 0 {
                    gd.black_stones = gd
                        .black_stones
                        .iter()
                        .filter(|s| !dead_black_stones.contains(s))
                        .cloned()
                        .collect();
                }
            }
        }
    }

    gd.white_stones.sort_by_key(|p| (p.x * gd.size) + p.y);
    gd.black_stones.sort_by_key(|p| (p.x * gd.size) + p.y);
    GameData {
        white_stones: gd
            .white_stones
            .iter()
            .map(|s| Point2 {
                x: s.x + 1,
                y: s.y + 1,
            })
            .collect(),
        black_stones: gd
            .black_stones
            .iter()
            .map(|s| Point2 {
                x: s.x + 1,
                y: s.y + 1,
            })
            .collect(),
        size: gd.size,
    }
}

#[cfg(test)]
mod test {
    use libremarkable::cgmath::Point2;
    use pretty_assertions::assert_eq;
    use std::fs;

    use crate::game_parse::{get_game_data, GameData};

    fn points(input: Vec<(u8, u8)>) -> Vec<Point2<u8>> {
        input.iter().map(|(x, y)| Point2 { x: *x, y: *y }).collect()
    }

    fn get_data(name: &str) -> GameData {
        let raw_data = fs::read(format!("src/test_data/{name}.sgf")).unwrap();
        let data = str::from_utf8(&raw_data).unwrap();
        get_game_data(&data)
    }

    #[test]
    fn basic_load() {
        let game_data = get_data("basic");
        assert_eq!(
            GameData {
                white_stones: points(vec![(7, 9)]),
                black_stones: points(vec![(4, 4), (4, 10), (10, 4), (10, 10)]),
                size: 13
            },
            game_data
        );
    }

    #[test]
    fn capture_load() {
        let game_data = get_data("one-capture");
        assert_eq!(
            GameData {
                white_stones: points(vec![
                    (4, 5),
                    (4, 6),
                    (4, 7),
                    (5, 3),
                    (6, 5),
                    (6, 6),
                    (7, 4),
                    (7, 6),
                    (8, 5)
                ]),
                black_stones: points(vec![
                    (3, 3),
                    (3, 5),
                    (3, 7),
                    (4, 3),
                    (5, 2),
                    (5, 7),
                    (7, 3),
                    (7, 7),
                    (8, 6),
                    (8, 7)
                ]),
                size: 9
            },
            game_data
        );
    }
}
