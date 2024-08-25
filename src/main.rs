#![feature(test)]
#![feature(portable_simd)]
#![feature(get_mut_unchecked)]
#![allow(dead_code)]

extern crate test;

#[macro_use]
extern crate lazy_static;

mod board;
mod gen_moves;
mod gen_tables;
mod moves;
mod search;
mod tt;
mod eval {
    pub use crate::gen_moves::eval::*;
}
mod uci {
    pub use crate::search::uci::*;
}

#[cfg(feature = "tuning")]
mod tuning;

use board::*;
use eval::*;
use gen_moves::*;
use gen_tables::*;
use moves::*;
use search::*;
use tt::*;
use uci::*;

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

#[cfg(feature = "tuning")]
fn main() {
    // tuning::positions_from_games("tuning_games2.pgn", "tuning_positions2.txt");
    println!(
        "{:#?}",
        tuning::tune("tuning_positions2.txt", "tuning_positions2.cache", &PARAMS)
    );
}

#[cfg(not(feature = "tuning"))]
fn main() {
    ucimanager(BufReader::new(io::stdin()));
    // let mut board = Board::from_fen("r1bqkbnr/pppp1ppp/8/8/3nP3/5N2/PPP2PPP/RNB1KB1R w KQkq - ");
    // let mut board = Board::from_fen(START_FEN);
    // let mut searcher = Searcher::new_single(1 << 24, false);

    // loop {
    // // let score = searcher.alphabeta(board.clone(), 316, 320, 3);
    // let score = searcher.alphabeta(board.clone(), -200000, 200000, 8);

    // board = board.do_move(searcher.get_best_move(&board).unwrap());
    // println!("{:?}", from_ibv(score.unwrap()));
    // println!("{}", board.to_fen(false));
    // println!("{:?}", board);

    // searcher.incr_time();
    // }

    // perftree();

    // let mut tt = TT::with_len(1024);
    // let board = Board::from_fen("1n2k3/4p3/5p1p/8/8/5P1P/4P3/4K1N1 w - -");
    // let mut generator = MoveGenerator::empty();

    // println!("{}", generator.eval(board, &mut tt));
}

use std::env;

fn perftree() {
    let args: Vec<_> = env::args().collect();
    let depth = args[1].parse::<usize>().unwrap();
    let mut board = Board::from_fen(&args[2]);

    if args.len() > 3 {
        for mov in args[3].split(' ') {
            board = board.do_move(mov.parse().unwrap());
        }
    }

    perftmanager(1 << 24, 4, board, depth);
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
