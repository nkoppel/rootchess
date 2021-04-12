#![feature(get_mut_unchecked)]
#![feature(test)]
extern crate test;

#[macro_use]
extern crate lazy_static;

mod gen_tables;
mod board;
mod gen_moves;
mod moves;
mod tt;
mod uci;
mod search;
mod eval { pub use crate::gen_moves::eval::*; }

use gen_tables::*;
use board::*;
use gen_moves::*;
use moves::*;
use tt::*;
use uci::*;
use search::*;
use eval::*;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

fn main() {
    let mut board = Board::from_fen(START_FEN);
    let mut generator = MoveGenerator::empty();
    let mut tt = TT::with_len(1024);

    loop {
        let mut tt = TT::with_len(1 << 24);
        let mut searcher = Searcher::new(tt.clone(), false);

        let score = searcher.search(board.clone(), 1, 9);

        board = board.do_move(searcher.get_best_move(&board).unwrap());
        println!("{:?}", board);
        println!("{:?}", generator.eval(board.clone(), &mut tt));
    }

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
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
