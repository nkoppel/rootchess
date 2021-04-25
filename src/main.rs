#![feature(test)]
#![feature(get_mut_unchecked)]
#![feature(duration_saturating_ops)]
extern crate test;

#[macro_use]
extern crate lazy_static;

mod gen_tables;
mod board;
mod gen_moves;
mod moves;
mod tt;
mod search;
mod eval { pub use crate::gen_moves::eval::*; }
mod uci { pub use crate::search::uci::*; }

use gen_tables::*;
use board::*;
use gen_moves::*;
use moves::*;
use tt::*;
use search::*;
use eval::*;
use uci::*;

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

fn main() {
    ucimanager(BufReader::new(io::stdin()));
    // let mut board = Board::from_fen("r1bqkbnr/pppp1ppp/8/8/3nP3/5N2/PPP2PPP/RNB1KB1R w KQkq - ");
    // let mut searcher = Searcher::new_single(1 << 24, false);

    // loop {
        // let score = searcher.alphabeta(board.clone(), 316, 320, 3);

        // board = board.do_move(searcher.get_best_move(&board).unwrap());
        // println!("{:?}", from_ibv(score.unwrap()));
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

// fn perftree() {
    // let args: Vec<_> = env::args().collect();
    // let depth = args[1].parse::<usize>().unwrap();
    // let mut board = Board::from_fen(&args[2]);

    // if args.len() > 3 {
        // for mov in args[3].split(' ') {
            // board = board.do_move(mov.parse().unwrap());
        // }
    // }

    // perftmanager(1 << 24, 4, board, depth);
// }

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
