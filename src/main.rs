#![feature(test)]
extern crate test;

#[macro_use]
extern crate lazy_static;

mod gen_tables;
mod board;
mod gen_moves;
mod moves;

use gen_tables::*;
use board::*;
use gen_moves::*;
use moves::*;

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

fn main() {
    perftree();

    // let mut board = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -");

    // board = board.do_move("a2a4".parse().unwrap());
    // let board2 = board.clone();
    // board = board.do_move("b4a3".parse().unwrap());
    // println!("{:?}", board);

    // let mut moves = Vec::new();
    // let generator = MoveGenerator::new(board2);

    // generator.gen_moves(|b| {moves.push(b); false});

    // assert!(moves.contains(&board));
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

    let generator = MoveGenerator::new(board.clone());
    let mut total = 0;

    generator.gen_moves(|b| {
        let res = perft(b.clone(), depth - 1);

        println!("{} {}", board.get_move(&b, false), res);
        total += res;

        false
    });

    println!();
    println!("{}", total);
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
