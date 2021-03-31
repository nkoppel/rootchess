#![feature(test)]
extern crate test;

#[macro_use]
extern crate lazy_static;

mod gen_tables;
mod board;
mod gen_moves;

use gen_tables::*;
use board::*;

fn main() {
    let board = Board::from_fen("rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq -");

    let squares =  board.to_squarewise();

    assert_eq!(board, Board::from_squarewise(&board.to_squarewise(), true));

    println!("{:?}", board);

    println!("{}", board.to_fen(false));
}
