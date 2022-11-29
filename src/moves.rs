use crate::gen_tables::*;
use crate::board::*;

const SQUARE: u16 = 0x3f;
const PIECE : u16 = 0x7;

#[derive(Clone, Copy, PartialEq, Default)]
pub struct Move(pub u16);

impl Move {
    pub const fn new() -> Self { Move(0) }

    pub fn pack(start: usize, end: usize, piece: usize) -> Self {
        Move (
            (start as u16 & SQUARE) << 10 |
            (end   as u16 & SQUARE) << 4  |
             piece as u16 & PIECE
        )
    }

    pub fn unpack(&self) -> (usize, usize, usize) {
        (self.start(), self.end(), self.piece())
    }

    pub fn start(&self) -> usize { (self.0 >> 10 & SQUARE) as usize }
    pub fn end  (&self) -> usize { (self.0 >> 4  & SQUARE) as usize }
    pub fn piece(&self) -> usize { (self.0       & PIECE ) as usize }

    pub fn from_uci(s: &str) -> Self {
        s.parse().unwrap()
    }
}

use std::fmt;

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", str_from_sq(self.start()))?;
        write!(f, "{}", str_from_sq(self.end()))?;

        let piece = self.piece();

        if piece != 0 {
            write!(f, "{}", &FEN_PIECES[piece + 8..piece + 9])?;
        }

        Ok(())
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

use std::str::FromStr;

impl FromStr for Move {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_ascii_lowercase();
        let start = sq_from_str(&s[0..2]);
        let end   = sq_from_str(&s[2..4]);
        let piece =
            if s.len() >= 5 {
                FEN_PIECES.find(&s[4..5]).unwrap_or(8) - 8
            } else {
                0
            };

        Ok(Move::pack(start, end, piece))
    }
}

impl Board {
    pub fn do_move(&self, mov: Move) -> Board {
        let mut out = self.clone();
        let (start, end, piece) = mov.unpack();
        let cur_occ = if self.black { self.black() } else { self.white() };
        out.black ^= true;

        // En Passant
        if self.pawns() & 1 << start != 0 &&
            self.takeable_empties() & 1 << end != 0
        {
            out.remove_takeable_empty();

            out.b ^= TABLES.en_pass
                [self.black as usize]
                [(end % 8 > start % 8) as usize]
                    << start as u32 % 8;

            out.update_hash(&self);

            out
        }
        // Castling (Chess960)
        else if self.kings() & 1 << start != 0 &&
            self.rooks() & 1 << end != 0 &&
            (self.white() & (1 << start | 1 << end)).count_ones() % 2 == 0
        {
            out.b ^= TABLES.castles[self.black as usize][start % 8][end % 8].2;

            out.b ^= u64x4::new(0,0,0,out.castling_rooks() & cur_occ);

            out.remove_takeable_empty();
            out.update_hash(&self);

            out
        }
        // Castling (Normal)
        else if self.kings() & 1 << start != 0 &&
            abs_diff(start, end) == 2
        {
            let end = if end > start {7} else {0};
            out.b ^= TABLES.castles[self.black as usize][start % 8][end % 8].2;

            out.b ^= u64x4::new(0,0,0,out.castling_rooks() & cur_occ);

            out.remove_takeable_empty();
            out.update_hash(&self);

            out
        }
        // Double-Moving Pawns
        else if self.pawns() & 1 << start != 0 &&
            (start as isize - end as isize).abs() == 16
        {
            let piece = self.b >> start as u32 & 1;

            out.remove_takeable_empty();

            out.b &= !(1 << start);
            out.b |= piece << end as u32;
            out.b |= u64x4::new(1,0,0,0) << (start + end) as u32 / 2;

            out.update_hash(&self);

            out
        }
        // Other moves
        else {
            let mut sq = self.b >> start as u32 & 1;

            if piece != 0 {
                let mut p = piece;

                if self.black {
                    p += 8;
                }

                sq = piece_to_sq(p as u8);
            }

            if self.castling_rooks() & 1 << start != 0 {
                sq ^= u64x4::new(0,0,0,1);
            }

            out.b &= !(1 << start | 1 << end);
            out.b |= sq << end as u32;

            if self.kings() & 1 << start != 0 {
                let cur_occ = if self.black {self.black()} else {self.white()};

                out.b ^= u64x4::new(0,0,0,self.castling_rooks() & cur_occ);
            }

            out.remove_takeable_empty();
            out.update_hash(&self);

            out
        }
    }

    pub fn get_move(&self, other: &Board, c960: bool) -> Move {
        let cur_occ = if self.black {self.black()} else {self.white()};
        let (board_diff, diff) = {
            let mut s = self.clone();
            let mut o = other.clone();

            s.remove_takeable_empty();
            o.remove_takeable_empty();

            let board_diff = s.b ^ o.b;

            s.b ^= u64x4::new(0,0,0, s.castling_rooks());
            o.b ^= u64x4::new(0,0,0, o.castling_rooks());

            (board_diff, (s.b ^ o.b).or())
        };

        let mut castle = 64;
        {
            let king_start = (self.kings() & cur_occ).trailing_zeros() as usize;

            for rook in LocStack(self.castling_rooks() & cur_occ) {
                let expected_diff = TABLES.castles[self.black as usize][king_start % 8][rook % 8].2;

                if board_diff & expected_diff.or() == expected_diff {
                    castle = rook;
                    break;
                }
            }
        }

        if castle < 64 {
            let start = (self.kings() & cur_occ).trailing_zeros() as usize;
            let mut end = castle;

            if !c960 {
                end = if end > start {5} else {1};

                if self.black {
                    end += 56;
                }
            }

            Move::pack(start, end, 0)
        } else if diff.count_ones() > 2 && diff & self.pawns() != 0 {
            let start = (self.pawns() & cur_occ & diff)
                .trailing_zeros() as usize;
            let end = self.takeable_empties().trailing_zeros() as usize;

            Move::pack(start, end, 0)
        } else {
            let start = (diff &  cur_occ).trailing_zeros() as usize;
            let end   = (diff & !cur_occ).trailing_zeros() as usize;
            let mut piece = 0;

            if (self.b >> start as u32) & 1 != (other.b >> end as u32) & 1 && 
                self.rooks() & 1 << start == 0
            {
                piece = other.get_square(end as u8) as usize;
            }

            Move::pack(start, end, piece)
        }
    }

    pub fn is_capture(&self, other: &Board) -> bool {
        let diff = {
            let mut s = self.clone();
            let mut o = other.clone();

            s.remove_takeable_empty();
            o.remove_takeable_empty();

            s.b ^= u64x4::new(0,0,0, s.castling_rooks());
            o.b ^= u64x4::new(0,0,0, o.castling_rooks());

            (s.b ^ o.b).or()
        };

        self.occ() & diff == diff &&
            (self .occ() & diff).count_ones() == 2 &&
            (other.occ() & diff).count_ones() == 1
    }
}

#[allow(unused_imports)]
use test::Bencher;

fn get_test_cases() -> Vec<(bool, Board, Move, Board)> {
    vec![
        (
            false,
            "7n/6P1/8/2pP4/5N2/8/P7/R3K3 w Q c6",
            vec![
                ("d5c6", "7n/6P1/2P5/8/5N2/8/P7/R3K3 b Q -"),
                ("g7h8r", "7R/8/8/2pP4/5N2/8/P7/R3K3 b Q -"),
                ("e1e2", "7n/6P1/8/2pP4/5N2/8/P3K3/R7 b - -"),
                ("a1a8", "R6n/6P1/8/2pP4/5N2/8/P7/4K3 b - -"),
                ("e1c1", "7n/6P1/8/2pP4/5N2/8/P7/2KR4 b - -"),
                ("a2a4", "7n/6P1/8/2pP4/P4N2/8/8/R3K3 b Q a3"),
            ]
        ),
        (
            true,
            "2kr4/pp5p/6p1/3pp3/P3n3/1PP3P1/1Q5P/R1K1B3 w A - 0 27",
            vec![
                ("c1a1", "2kr4/pp5p/6p1/3pp3/P3n3/1PP3P1/1Q5P/2KRB3 b - - 0 27")
            ]
        ),
        (
            true,
            "1rnbbkrn/p1p2ppp/8/1p1B4/1P1P4/3N4/1R1P1PPP/4BKRN b Ggb - 0 10",
            vec![
                ("f8g8", "1rnbbrkn/p1p2ppp/8/1p1B4/1P1P4/3N4/1R1P1PPP/4BKRN w G - 1 11")
            ]
        )
    ]
        .into_iter()
        .flat_map(|(c960, fen1, moves)| {
            let board = Board::from_fen(fen1);

            moves
                .into_iter()
                .map(move |(m, fen2)| (c960, board.clone(), m.parse().unwrap(), Board::from_fen(fen2)))
        })
        .collect()
}

#[test]
fn t_moves() {
    for (_, board, mov, board2) in get_test_cases() {
        assert_eq!(board.do_move(mov), board2);
    }

    for (c960, board, mov, board2) in get_test_cases() {
        assert_eq!(board.get_move(&board2, c960), mov);
    }
}
