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
mod eval { pub use crate::gen_moves::eval; }

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
    // let board = Board::from_fen("8/8/8/8/1p6/2k5/K1p5/8 b - -");
    // let mut generator = MoveGenerator::empty();
    // let mut tt = TT::with_len(10);

    // println!("{}", generator.eval(board, &mut tt));
    perftree();

    // let mut board = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -");
    // let mut board = Board::from_fen(START_FEN);
    // let mut generator = MoveGenerator::new(board);

    // generator.gen_moves();

    // println!("{:?}", generator.moves);
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
