#![feature(test)]
extern crate test;

mod gen_tables;
mod board;
mod gen_moves;

use gen_tables::*;
use board::*;

fn main() {
    let hasher = Hasher::new();
    let board = Board::from_fen("rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq -", &hasher);
    let tables = Tables::new();

    let squares =  board.to_squarewise();

    assert_eq!(board, Board::from_squarewise(&board.to_squarewise(), true, &hasher));

    for y in (0..8).rev() {
        for x in (0..8).rev() {
            let sq = squares[x + y * 8];

            if sq == 0 {
                print!("_ ");
            } else {
                print!("{:X} ", sq);
            }
        }
        println!();
    }

    for (sq, moves) in board.gen_moves_bits(&tables) {
        println!("{}", sq);
        print_board(moves);
    }

    println!("{}", board.to_fen(false));
}
