use crate::gen_tables::*;
use crate::board::*;
use crate::gen_moves::*;

const SQUARE: u16 = 0x3f;
const PIECE : u16 = 0xf;

#[derive(Clone, Debug, PartialEq)]
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
        (self.get_start(), self.get_end(), self.get_piece())
    }

    pub fn get_start(&self) -> usize { (self.0 >> 10 & SQUARE) as usize }
    pub fn get_end  (&self) -> usize { (self.0 >> 4  & SQUARE) as usize }
    pub fn get_piece(&self) -> usize { (self.0       & PIECE ) as usize }

    pub fn from_uci(s: &str) -> Self {
        s.parse().unwrap()
    }
}

use std::fmt;

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", str_from_sq(self.get_start()));
        write!(f, "{}", str_from_sq(self.get_end()));

        let piece = self.get_piece();

        if piece != 0 {
            write!(f, "{}", &FEN_PIECES[piece + 8..piece + 9]);
        }

        Ok(())
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
        out.black ^= true;

        if self.pawns() & 1 << start != 0 &&
            self.takeable_empties() & 1 << end != 0
        {
            out.b ^= TABLES.en_pass
                [self.black as usize]
                [(end % 8 > start % 8) as usize]
                    << start as u32 % 8;

            out.update_hash(&self);

            out
        }
        else if self.kings() & 1 << start != 0 &&
            self.rooks() & 1 << end != 0 &&
            (self.white() & (1 << start | 1 << end)).count_ones() % 2 == 0
        {
            out.b ^= TABLES.castles[self.black as usize][start][end].2;

            out.remove_takeable_empty();
            out.update_hash(&self);

            out
        }
        else if self.kings() & 1 << start != 0 &&
            abs_diff(start, end) == 2
        {
            let end = if end > start {7} else {0};
            out.b ^= TABLES.castles[self.black as usize][start][end].2;

            out.remove_takeable_empty();
            out.update_hash(&self);

            out
        }
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
        let diff = {
            let mut s = self.clone();
            let mut o = other.clone();

            s.remove_takeable_empty();
            o.remove_takeable_empty();

            s.b ^= u64x4::new(0,0,0, s.castling_rooks());
            o.b ^= u64x4::new(0,0,0, o.castling_rooks());

            (s.b ^ o.b).or()
        };

        if (cur_occ & diff).count_ones() > 1 {
            let start   = (self.kings() & cur_occ).trailing_zeros() as usize;
            let mut end = (self.rooks() & cur_occ & diff)
                .trailing_zeros() as usize;

            if !c960 {
                end = end.clamp(1, 5);
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
}

mod tests {
    use super::*;
    use test::Bencher;

    fn get_test_cases() -> Vec<(Move, Board)> {
        vec![
            ("d5c6", "7n/6P1/2P5/8/5N2/8/P7/R3K3 b Q -"),
            ("g7h8r", "7R/8/8/2pP4/5N2/8/P7/R3K3 b Q -"),
            ("e1e2", "7n/6P1/8/2pP4/5N2/8/P3K3/R7 b - -"),
            ("a1a8", "R6n/6P1/8/2pP4/5N2/8/P7/4K3 b - -"),
            ("e1c1", "7n/6P1/8/2pP4/5N2/8/P7/2KR4 b - -"),
            ("a2a4", "7n/6P1/8/2pP4/P4N2/8/8/R3K3 b Q a3"),
        ]
            .into_iter()
            .map(|(m, b)| (m.parse().unwrap(), Board::from_fen(b)))
            .collect()
    }

    #[test]
    fn t_moves() {
        let board = Board::from_fen("7n/6P1/8/2pP4/5N2/8/P7/R3K3 w Q c6");

        for (mov, board2) in get_test_cases() {
            assert_eq!(board.do_move(mov), board2);
        }

        for (mov, board2) in get_test_cases() {
            assert_eq!(board.get_move(&board2, false), mov);
        }
    }
}
