mod gen_tables;
mod board;

use gen_tables::*;
use board::*;

fn main() {
    println!("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3");

    let hasher = Hasher::new();
    let board = Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3", &hasher);

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
    println!("{}", board.to_fen(false));
}
