pub mod consts;
pub mod position;
pub mod movegen;
pub mod hash;
pub mod eval;
pub mod search;

use std::io::stdin;
use std::time::Instant;
use consts::{VERSION, AUTHOR, CastleRights, EMPTY, WHITE, BLACK};
use hash::{tt_clear, tt_resize, zobrist, kt_clear};
use position::{POS, MoveList, do_move, undo_move, GameState};
use movegen::{gen_moves, All};
use search::{DEPTH, TIME, go};

const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
const LASKER: &str = "8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1";

pub const _POSITIONS: [&str; 12] = [
    STARTPOS, LASKER, KIWIPETE,
    // Standard low depth mate puzzles
    "rn5r/pp3kpp/2p1R3/5p2/3P4/2B2N2/PPP3PP/2K4n w - - 1 17",
    "4r1rk/pp4pp/2n5/8/6Q1/7R/1qPK1P1P/3R4 w - - 0 28",
    "2r1rbk1/1R3R1N/p3p1p1/3pP3/8/q7/P1Q3PP/7K b - - 0 25",
    // Positions that catch pruning methods out
    "8/2krR3/1pp3bp/6p1/PPNp4/3P1PKP/8/8 w - - 0 1",
    "1Q6/8/8/8/2k2P2/1p6/1B4K1/8 w - - 3 63",
    "3r2k1/pp3ppp/4p3/8/QP6/P1P5/5KPP/7q w - - 0 27",
    "1q1r3k/3P1pp1/ppBR1n1p/4Q2P/P4P2/8/5PK1/8 w - - 0 1",
    "1n3r2/3k2pp/pp1P4/1p4b1/1q3B2/5Q2/PPP2PP1/R4RK1 w - - 0 1",
    "7K/8/k1P5/7p/8/8/8/8 w - - 0 1"
];

fn main() {
    println!("akimbo, created by Jamie Whiting");
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        let commands: Vec<&str> = input.split(' ').map(|v| v.trim()).collect();
        if commands[0] == "uci" {uci_run()}
    }
}

fn performance() {
    unsafe {
    TIME = 1000;
    let now = Instant::now();
    for fen  in _POSITIONS {
        kt_clear();
        parse_fen(fen);
        println!("===Search Report===");
        println!("fen: {}", fen);
        go();
        println!(" ");
    }
    println!("Total time: {}ms", now.elapsed().as_millis());
    ucinewgame();
    }
}

fn perft(depth_left: u8) -> u64 {
    if depth_left == 0 { return 1 }
    let mut moves = MoveList::default();
    gen_moves::<All>(&mut moves);
    let mut positions: u64 = 0;
    for m_idx in 0..moves.len {
        let m = moves.list[m_idx];
        let invalid = do_move(m);
        if invalid { continue }
        let score = perft(depth_left - 1);
        positions += score;
        undo_move();
    }
    positions
}

fn uci_run() {
    parse_fen(STARTPOS);
    tt_resize(128 * 1024 * 1024);
    println!("id name akimbo {}", VERSION);
    println!("id author {}", AUTHOR);
    println!("uciok");
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        let commands: Vec<&str> = input.split(' ').map(|v| v.trim()).collect();
        run_commands(commands);
    }
}

fn run_commands(commands: Vec<&str>) {
    match commands[0] {
        "isready" => println!("readyok"),
        "ucinewgame" => ucinewgame(),
        "go" => parse_go(commands),
        "position" => parse_position(commands),
        "performance" => performance(),
        _ => {},
    };
}

fn ucinewgame() {
    parse_fen(STARTPOS);
    tt_clear();
    kt_clear();
}

fn parse_go( commands: Vec<&str>) {
    #[derive(PartialEq)]
    enum Tokens {None, Depth, Perft, Movetime}
    let mut token = Tokens::None;
    let mut perft_depth = 0;
    for command in commands {
        match command {
            "depth" => token = Tokens::Depth,
            "movetime" => token = Tokens::Movetime,
            "perft" => token = Tokens::Perft,
            _ => {
                match token {
                    Tokens::None => {},
                    Tokens::Depth => unsafe{DEPTH = command.parse::<i8>().unwrap_or(1)},
                    Tokens::Movetime => unsafe{TIME = command.parse::<i64>().unwrap_or(1000) as u128 - 10}
                    Tokens::Perft => perft_depth = command.parse::<u8>().unwrap_or(1),
                }
            },
        }
    }
    if token == Tokens::Perft {
        let now = Instant::now();
        let mut total = 0;
        for d in 0..perft_depth {
            let count = perft(d + 1);
            total += count;
            println!("info depth {} nodes {}", d + 1, count)
        }
        let elapsed = now.elapsed().as_micros();
        println!("Leaf count: {total} ({:.2} ML/sec)", total as f64 / elapsed as f64);
    } else {
        go();
    }
}

fn parse_position(commands: Vec<&str>) {
    enum Tokens {Nothing, Fen, Moves}
    let mut fen = String::from("");
    let mut moves: Vec<String> = Vec::new();
    let mut token = Tokens::Nothing;
    for command in commands {
        match command {
            "position" => (),
            "startpos" => parse_fen(STARTPOS),
            "kiwipete" => parse_fen(KIWIPETE),
            "lasker" => parse_fen(LASKER),
            "fen" => token = Tokens::Fen,
            "moves" => token = Tokens::Moves,
            _ => match token {
                Tokens::Nothing => {},
                Tokens::Fen => {
                    fen.push_str(command);
                    fen.push(' ');
                }
                Tokens::Moves => moves.push(command.to_string()),
            },
        }
    }
    if !fen.is_empty() {parse_fen(&fen)}
    for m in moves {do_move(uci_to_u16(&m));}
}

/// UCI MOVE FORMAT
fn idx_to_sq(idx: u16) -> String {
    let rank = idx >> 3;
    let file = idx & 7;
    let srank = (rank + 1).to_string();
    let sfile = FILES[file as usize];
    format!("{sfile}{srank}")
}
fn sq_to_idx(sq: &str) -> u16 {
    let chs: Vec<char> = sq.chars().collect();
    let file: u16 = match FILES.iter().position(|&ch| ch == chs[0]) {
        Some(res) => res as u16,
        None => 0,
    };
    let rank = chs[1].to_string().parse::<u16>().unwrap_or(0) - 1;
    8 * rank + file
}
const PROMOS: [&str; 4] = ["n","b","r","q"];
const PROMO_BIT: u16 = 0b1000_0000_0000_0000;
pub fn u16_to_uci(m: &u16) -> String {
    let mut promo = "";
    if m & PROMO_BIT > 0 {
        promo = PROMOS[((m >> 12) & 0b11) as usize];
    }
    format!("{}{}{} ", idx_to_sq((m >> 6) & 0b111111), idx_to_sq(m & 0b111111), promo)
}
const TWELVE: u16 = 0b0000_1111_1111_1111;
pub fn uci_to_u16(m: &str) -> u16 {
    let l = m.len();
    let from = sq_to_idx(&m[0..2]);
    let to = sq_to_idx(&m[2..4]);
    let mut no_flags = (from << 6) | to;
    if l == 5 {
        no_flags |= match m.chars().nth(4).unwrap() {
            'n' => 0b1000_0000_0000_0000,
            'b' => 0b1001_0000_0000_0000,
            'r' => 0b1010_0000_0000_0000,
            'q' => 0b1011_0000_0000_0000,
            _ => 0,
        }
    }
    let mut possible_moves = MoveList::default();
    gen_moves::<All>(&mut possible_moves);
    for m_idx in 0..possible_moves.len {
        let um = possible_moves.list[m_idx];
        if no_flags & TWELVE == um & TWELVE {
            if l < 5 {
                return um;
            }
            if no_flags & !TWELVE == um & 0b1011_0000_0000_0000 {
                return um;
            }
        }
    }
    panic!("")
}


// FEN
const FILES: [char; 8] = ['a','b','c','d','e','f','g','h'];
const PIECES: [char; 12] = ['P','N','B','R','Q','K','p','n','b','r','q','k'];
pub fn parse_fen(s: &str) {
    unsafe {
    let vec: Vec<&str> = s.split_whitespace().collect();
    POS.pieces = [0;6];
    POS.squares = [EMPTY as u8; 64];
    POS.sides = [0; 2];
    let mut idx: usize = 63;
    let rows: Vec<&str> = vec[0].split('/').collect();
    for row in rows {
        for ch in row.chars().rev() {
            if ch == '/' { continue }
            if !ch.is_numeric() {
                let idx2 = PIECES.iter().position(|&element| element == ch).unwrap_or(6);
                let (col, pc) = ((idx2 > 5) as usize, idx2 - 6 * ((idx2 > 5) as usize));
                toggle!(col, pc, 1 << idx);
                POS.squares[idx] = pc as u8;
                idx -= (idx > 0) as usize;
            } else {
                let len = ch.to_string().parse::<usize>().unwrap_or(8);
                idx -= (idx >= len) as usize * len;
            }
        }
    }
    POS.side_to_move = match vec[1] { "w" => WHITE, "b" => BLACK, _ => panic!("") };
    let mut castle_rights = CastleRights::NONE;
    for ch in vec[2].chars() {
        castle_rights |= match ch {'Q' => CastleRights::WHITE_QS, 'K' => CastleRights::WHITE_KS, 'q' => CastleRights::BLACK_QS, 'k' => CastleRights::BLACK_KS, _ => 0,};
    }
    let en_passant_sq = if vec[3] == "-" {0} else {
        let arr: Vec<char> = vec[3].chars().collect();
        let rank: u16 = arr[1].to_string().parse::<u16>().unwrap_or(0) - 1;
        let file = FILES.iter().position(|&c| c == arr[0]).unwrap_or(0);
        8 * rank + file as u16
    };
    let halfmove_clock = vec[4].parse::<u8>().unwrap_or(0);
    let (phase, mg, eg) = eval::calc();
    POS.state = GameState {zobrist: 0, phase, mg, eg,en_passant_sq, halfmove_clock, castle_rights};
    POS.fullmove_counter = vec[5].parse::<u16>().unwrap_or(1);
    POS.state.zobrist = zobrist::calc();
    POS.stack.clear();
    }
}