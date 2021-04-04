#![feature(test)]
extern crate test;

#[macro_use]
extern crate lazy_static;

mod gen_tables;
mod board;
mod gen_moves;
mod moves;
mod search;

use gen_tables::*;
use board::*;
use gen_moves::*;
use moves::*;
use search::*;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

fn main() {
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

    let mut generator = MoveGenerator::new(board.clone());
    let mut searcher = Searcher::new(22);
    let mut total = 0;

    generator.gen_moves();

    for b in generator.moves {
        let res = searcher.perft(b.clone(), depth - 1);

        println!("{} {}", board.get_move(&b, false), res);
        total += res;
    }

    println!();
    println!("{}", total);
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
