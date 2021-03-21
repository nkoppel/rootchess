use crate::gen_tables::*;
use crate::board::*;

pub const START_FEN: &str =
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn gen_rook_moves(tables: &Tables, sq: usize, mut occ: u64) -> u64 {
    let (mask, magic, offset) = tables.rook[sq];

    occ &= mask;
    occ = occ.overflowing_mul(magic).0;
    occ >>= 52;
    occ += offset;

    tables.magic[occ as usize]
}

fn gen_bishop_moves(tables: &Tables, sq: usize, mut occ: u64) -> u64 {
    let (mask, magic, offset) = tables.bishop[sq];

    occ &= mask;
    occ = occ.overflowing_mul(magic).0;
    occ >>= 55;
    occ += offset;

    tables.magic[occ as usize]
}

#[derive(Clone, Debug, PartialEq)]
pub enum Move {
    Basic(u8, u8),         // sq1, sq2
    En_passant(u8, u8),    // sq1, sq2
    Castle(u8),            // file of rook
    Promotion(u8, u8, u8), // piece, sq1, sq2
}

#[derive(Clone, Debug, PartialEq)]
pub struct Moves {
    pub bits: Vec<(u8, u64)>,
    pub others: Vec<Move>
}

impl Board {
    fn get_checks(&self, tables: &Tables) -> u64 {
        let occ = self.occ();
        let (cur_occ, opp_occ, pawn_takes) =
            if self.black {
                (self.black(), self.white(), &tables.black_pawn_takes)
            } else {
                (self.white(), self.black(), &tables.white_pawn_takes)
            };

        let king_loc = (self.kings() & cur_occ).trailing_zeros() as usize;
        let mut out = 0;

        out |= pawn_takes[king_loc] & opp_occ & self.pawns();
        out |= tables.knight[king_loc] & opp_occ & self.knights();
        out |= gen_bishop_moves(tables, king_loc, occ) & opp_occ &
            (self.bishops() | self.queens());
        out |= gen_rook_moves(tables, king_loc, occ) & opp_occ &
            (self.rooks() | self.queens());

        out
    }

    fn get_threatened(&self, tables: &Tables) -> u64 {
        let mut out = 0;

        let (cur_occ, opp_occ, pawn_takes) =
            if self.black {
                (self.black(), self.white(), &tables.white_pawn_takes)
            } else {
                (self.white(), self.black(), &tables.black_pawn_takes)
            };

        let mut board = self.clone();
        let king = self.kings() & cur_occ;
        board.b &= !king;

        let occ = board.occ();

        for sq in LocStack(board.pawns() & opp_occ) {
            out |= pawn_takes[sq];
        }

        for sq in LocStack(board.knights() & opp_occ) {
            out |= tables.knight[sq];
        }

        for sq in LocStack(board.kings() & opp_occ) {
            out |= tables.king[sq];
        }

        for sq in LocStack((board.bishops() | board.queens()) & opp_occ) {
            out |= gen_bishop_moves(tables, sq, occ);
        }

        for sq in LocStack((board.rooks() | board.queens()) & opp_occ) {
            out |= gen_rook_moves(tables, sq, occ);
        }

        out
    }

    fn get_pins(&self, tables: &Tables) -> Vec<u64> {
        let occ = self.occ();
        let (cur_occ, opp_occ, pawn_takes) =
            if self.black {
                (self.black(), self.white(), &tables.white_pawn_takes)
            } else {
                (self.white(), self.black(), &tables.black_pawn_takes)
            };

        let king_loc = (self.kings() & cur_occ).trailing_zeros() as usize;
        let mut out = vec![u64::MAX; 64];

        let bishop = gen_bishop_moves(tables, king_loc, opp_occ);
        let rook = gen_rook_moves(tables, king_loc, opp_occ);

        for pin in LocStack(bishop & opp_occ & (self.bishops() | self.queens()))
        {
            let moves =
                bishop & gen_bishop_moves(tables, pin, opp_occ) | (1 << pin);

            if (moves & cur_occ).count_ones() == 1 {
                out[pin] = moves;
            }
        }

        for pin in LocStack(rook & opp_occ & (self.rooks() | self.queens()))
        {
            let moves =
                rook & gen_rook_moves(tables, pin, opp_occ) | (1 << pin);

            if (moves & cur_occ).count_ones() == 1 {
                out[pin] = moves;
            }
        }

        out
    }

    fn get_blocks(&self, tables: &Tables) -> (u64, u64) {
        let occ = self.occ();
        let (cur_occ, opp_occ) =
            if self.black {
                (self.black(), self.white())
            } else {
                (self.white(), self.black())
            };

        let king_loc = (self.kings() & cur_occ).trailing_zeros() as usize;
        let checks = self.get_checks(tables);

        match checks.count_ones() {
            0 => return (checks, u64::MAX),
            1 => {},
            _ => return (checks, 0)
        }

        let check_loc = checks.trailing_zeros() as usize;

        let rook = gen_rook_moves(tables, king_loc, occ);

        if rook & checks != 0 {
            return (checks,
                gen_rook_moves(tables, check_loc, occ) & rook | checks);
        }

        let bishop = gen_bishop_moves(tables, king_loc, occ);

        if bishop & checks != 0 {
            return (checks,
                gen_bishop_moves(tables, check_loc, occ) & bishop | checks);
        }

        (checks, checks)
    }

    pub fn gen_moves_bits(&self, tables: &Tables) -> Vec<(u8, u64)> {
        let mut out = Vec::with_capacity(16);

        let occ = self.occ();
        let (cur_occ, opp_occ, pawn_shift, pawn_mask1, pawn_mask2, pawn_takes) =
            if self.black {
                (self.black(), self.white(),
                    Box::new(|x| x >> 8) as Box<dyn Fn(u64) -> u64>,
                    0xffff000000000000, 0xffffffffffff0000,
                    &tables.black_pawn_takes)
            } else {
                (self.white(), self.black(),
                    Box::new(|x| x << 8) as Box<dyn Fn(u64) -> u64>,
                    0x000000000000ffff, 0x0000ffffffffffff,
                    &tables.white_pawn_takes)
            };

        let threats = self.get_threatened(tables);
        let (checks, blocks) = self.get_blocks(tables);
        let pins = self.get_pins(tables);

        for sq in LocStack(self.kings() & cur_occ) {
            out.push((sq as u8, tables.king[sq] & !cur_occ & !threats));
        }

        if checks.count_ones() > 1 {
            return out;
        }

        for sq in LocStack(self.pawns() & cur_occ & pawn_mask1) {
            let mut moves;

            moves = pawn_shift(1 << sq) & !occ;
            moves |= pawn_shift(moves);
            moves &= !occ;

            moves |= pawn_takes[sq] & (opp_occ | self.takeable_empties());

            moves &= blocks;
            moves &= pins[sq];

            out.push((sq as u8, moves));
        }

        for sq in LocStack(self.pawns() & cur_occ & pawn_mask2) {
            let mut moves = pawn_shift(1 << sq) & !occ;
            moves |= pawn_takes[sq] & opp_occ;

            moves &= blocks;
            moves &= pins[sq];

            out.push((sq as u8, moves));
        }

        for sq in LocStack(self.knights() & cur_occ) {
            let mut moves = tables.knight[sq] & !cur_occ;

            moves &= blocks;
            moves &= pins[sq];

            out.push((sq as u8, moves));
        }

        for sq in LocStack(self.bishops() & cur_occ) {
            let mut moves = gen_bishop_moves(tables, sq, occ) & !cur_occ;

            moves &= blocks;
            moves &= pins[sq];

            out.push((sq as u8, moves));
        }

        for sq in LocStack(self.rooks() & cur_occ) {
            let mut moves = gen_rook_moves(tables, sq, occ) & !cur_occ;

            moves &= blocks;
            moves &= pins[sq];

            out.push((sq as u8, moves));
        }

        for sq in LocStack(self.queens() & cur_occ) {
            let mut moves = 0;

            moves |= gen_bishop_moves(tables, sq, occ);
            moves |= gen_rook_moves(tables, sq, occ);
            moves &= !cur_occ;

            moves &= blocks;
            moves &= pins[sq];

            out.push((sq as u8, moves));
        }

        out.retain(|(_, x)| *x != 0);

        out
    }
}

mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn t_get_checks() {
        let tables = Tables::new();
        let hasher = Hasher::new();
        let mut board1 = Board::from_fen(
            "r7/q7/8/8/8/1nb5/2n5/K1P4r w - -",
        &hasher);

        assert_eq!(board1.get_checks(&tables), 0x0080000000602000);
    }

    #[test]
    fn t_get_threatened() {
        let tables = Tables::new();
        let hasher = Hasher::new();
        let mut board1 = Board::from_fen(
            "8/b5k1/8/3q1p2/8/1P1n1K2/8/7r w - -",
        &hasher);

        assert_eq!(board1.get_threatened(&tables), 0xd7557fed7f5d47ff);
    }

    #[test]
    fn t_get_blocks() {
        let tables = Tables::new();
        let hasher = Hasher::new();

        let mut board = Board::from_fen("8/8/8/8/8/2b5/8/K7 w - -", &hasher); 
        assert_eq!(board.get_blocks(&tables).1, 0x204000);

        board = Board::from_fen("8/8/r7/8/8/8/8/K7 w - -", &hasher); 
        assert_eq!(board.get_blocks(&tables).1, 0x0000808080808000);

        board = Board::from_fen("8/8/r7/8/8/2b5/8/K7 w - -", &hasher); 
        assert_eq!(board.get_blocks(&tables).1, 0);

        board = Board::from_fen("8/8/8/8/8/1n6/8/K7 w - -", &hasher); 
        assert_eq!(board.get_blocks(&tables).1, 0x400000);

        board = Board::from_fen("8/8/8/8/8/8/1p6/K7 w - -", &hasher); 
        assert_eq!(board.get_blocks(&tables).1, 0x4000);

        board = Board::from_fen("8/8/8/8/8/2n5/8/K7 w - -", &hasher); 
        assert_eq!(board.get_blocks(&tables).1, u64::MAX);
    }

    #[bench]
    fn b_init_hash(b: &mut Bencher) {
        let hasher = Hasher::new();
        let mut board = Board::from_fen(START_FEN, &hasher);

        b.iter(|| {
            board.init_hash(&hasher);
            board.hash
        });
    }

    #[bench]
    fn b_update_hash(b: &mut Bencher) {
        let hasher = Hasher::new();
        let mut board1 = Board::from_fen(START_FEN, &hasher);
        let mut board2 = Board::from_fen(
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        &hasher);

        b.iter(|| {
            board2.update_hash(&board1, &hasher);
            board2.hash
        });
    }
}
