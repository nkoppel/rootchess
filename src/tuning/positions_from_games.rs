use shakmaty::{
    Chess,
    Position,
    Setup,
    san::SanPlus,
    fen::fen,
    Color,
    Outcome,
};

use pgn_reader::{Visitor, Skip, RawComment, RawHeader, BufferedReader};

type Positions = Vec<(f32, Chess)>;

struct PgnVisitor {
    skip: bool,
    positions: Positions,
    board: Chess,
}

impl Visitor for PgnVisitor {
    type Result = Positions;

    fn begin_game(&mut self) {
        self.skip = false;
        self.positions.clear();
        self.board = Chess::default();
    }

    fn header(&mut self, key: &[u8], _: RawHeader) {
        if key == b"Termination" {
            self.skip = true;
        }
    }

    fn end_headers(&mut self) -> Skip {
        Skip(self.skip)
    }

    fn san(&mut self, sanplus: SanPlus) {
        self.positions.push((0.5, self.board.clone()));

        let mov = sanplus.san.to_move(&self.board).unwrap();
        self.board = self.board.clone().play(&mov).unwrap();
    }

    fn comment(&mut self, comment: RawComment<'_>) {
        let comment = std::str::from_utf8(comment.as_bytes()).unwrap();

        // Filter out book moves
        if comment == "book" {
            self.positions.pop().unwrap();
        } else {
            let ind = comment.find('/').expect(comment);

            // Filter out checkmate positions
            if &comment[1..ind] == "256.00" {
                self.positions.pop().unwrap();
            }
        }
    }

    fn outcome(&mut self, outcome: Option<Outcome>) {
        let outcome_num =
            match outcome {
                Some(Outcome::Decisive {winner: color}) => {
                    if color == Color::White {
                        1.
                    } else {
                        0.
                    }
                },
                Some(Outcome::Draw) => 0.5,
                None => panic!("Game did not include outcome!"),
            };

        for (num, board) in &mut self.positions {
            if board.turn() == Color::White {
                *num = outcome_num;
            } else {
                *num = 1. - outcome_num;
            }
        }
    }

    fn begin_variation(&mut self) -> Skip { Skip(true) }

    fn end_game(&mut self) -> Positions {
        std::mem::replace(&mut self.positions, Vec::new())
    }
}

use std::fs::File;
use std::io::Write;

use crate::board::*;
use crate::tt::*;
use crate::gen_moves::*;
use crate::search::*;

pub fn positions_from_games(file1: &str, file2: &str) {
    let     read  = File::open  (file1).unwrap();
    let mut write = File::create(file2).unwrap();

    let mut visitor = PgnVisitor {
        skip: false,
        positions: Vec::new(),
        board: Chess::default(),
    };

    let iter = BufferedReader::new(read)
        .into_iter(&mut visitor)
        .map(|p| p.unwrap())
        .flatten();

    let mut pawn_tt = TT::with_len(1024);
    let mut searcher = Searcher::new_single(0, false);
    let mut generator = MoveGenerator::empty();

    for (outcome, board) in iter {
        let board2 = Board::from_fen(&fen(&board));

        let eval = generator.eval(board2.clone(), &mut pawn_tt);
        let quiesce = searcher.quiesce(board2, -2000000, 2000000) / 4;

        if (eval - quiesce).abs() < 25 {
            writeln!(&mut write, "{} {}", outcome, fen(&board)).unwrap();
        }
    }
}
