use crate::gen_tables::*;
use crate::board::*;

#[inline]
fn gen_rook_moves(sq: usize, mut occ: u64) -> u64 {
    let (mask, magic, offset) = TABLES.rook[sq];

    occ &= mask;
    occ = occ.overflowing_mul(magic).0;
    occ >>= 52;
    occ += offset;

    TABLES.magic[occ as usize]
}

#[inline]
fn gen_bishop_moves(sq: usize, mut occ: u64) -> u64 {
    let (mask, magic, offset) = TABLES.bishop[sq];

    occ &= mask;
    occ = occ.overflowing_mul(magic).0;
    occ >>= 55;
    occ += offset;

    TABLES.magic[occ as usize]
}

#[derive(Clone, Debug, PartialEq)]
pub enum Move {
    Basic(u8, u8),         // sq1, sq2
    En_passant(u8, u8),    // sq1, sq2
    Castle(u8),            // file of rook
    Promotion(u8, u8, u8), // piece, sq1, sq2
}

pub use Move::*;

pub struct MoveGenerator {
    board: Board,
    pins: Vec<u64>,
    cur_occ: u64,
    opp_occ: u64,
    cur_pawn_takes: &'static [u64],
    opp_pawn_takes: &'static [u64],
    checks: u64,
    blocks: u64,
    threatened: u64,
    moves_bits: Vec<(u8, u64)>,
    moves_special: Vec<Move>,
}

impl MoveGenerator {
    pub fn empty() -> Self {
        Self {
            board: Board::new(),
            pins: vec![0; 64],
            cur_occ: 0,
            opp_occ: 0,
            cur_pawn_takes: &TABLES.white_pawn_takes,
            opp_pawn_takes: &TABLES.black_pawn_takes,
            checks: 0,
            blocks: 0,
            threatened: 0,
            moves_bits: Vec::with_capacity(16),
            moves_special: Vec::with_capacity(40),
        }
    }

    pub fn new(board: Board) -> Self {
        let mut out = Self::empty();

        out.set_board(board);
        out
    }

    pub fn set_board(&mut self, board: Board) {
        self.board = board;
        if self.board.black {
            self.cur_occ = self.board.black();
            self.opp_occ = self.board.white();
            self.cur_pawn_takes = &TABLES.black_pawn_takes;
            self.opp_pawn_takes = &TABLES.white_pawn_takes;
        } else {
            self.cur_occ = self.board.white();
            self.opp_occ = self.board.black();
            self.cur_pawn_takes = &TABLES.white_pawn_takes;
            self.opp_pawn_takes = &TABLES.black_pawn_takes;
        }
        self.set_threatened();
        self.set_pins();
        self.set_blocks();
    }

    fn get_threats(&self, sq: usize) -> u64 {
        let mut out = 0;
        let occ = self.board.occ();

        out |= self.cur_pawn_takes[sq] & self.opp_occ & self.board.pawns();
        out |= TABLES.knight[sq] & self.opp_occ & self.board.knights();
        out |= gen_bishop_moves(sq, occ) & self.opp_occ &
            (self.board.bishops() | self.board.queens());
        out |= gen_rook_moves(sq, occ) & self.opp_occ &
            (self.board.rooks() | self.board.queens());

        out
    }

    fn set_threatened(&mut self) {
        let mut out = 0;

        let mut board = self.board.clone();
        let king = self.board.kings() & self.cur_occ;
        board.b &= !king;

        let occ = board.occ();

        for sq in LocStack(board.pawns() & self.opp_occ) {
            out |= self.opp_pawn_takes[sq];
        }

        for sq in LocStack(board.knights() & self.opp_occ) {
            out |= TABLES.knight[sq];
        }

        for sq in LocStack(board.kings() & self.opp_occ) {
            out |= TABLES.king[sq];
        }

        for sq in LocStack((board.bishops() | board.queens()) & self.opp_occ) {
            out |= gen_bishop_moves(sq, occ);
        }

        for sq in LocStack((board.rooks() | board.queens()) & self.opp_occ) {
            out |= gen_rook_moves(sq, occ);
        }

        self.threatened = out
    }

    fn set_pins(&mut self) {
        let occ = self.board.occ();

        let king_loc = (self.board.kings() & self.cur_occ).trailing_zeros() as usize;
        for p in self.pins.iter_mut() {
            *p = u64::MAX;
        }

        let bishop = gen_bishop_moves(king_loc, self.opp_occ);
        let rook = gen_rook_moves(king_loc, self.opp_occ);

        for pin in LocStack(bishop & self.opp_occ & (self.board.bishops() | self.board.queens()))
        {
            let moves =
                bishop & gen_bishop_moves(pin, self.opp_occ) | (1 << pin);

            let piece = moves & self.cur_occ;

            if piece.count_ones() == 1 {
                self.pins[piece.trailing_zeros() as usize] = moves;
            }
        }

        for pin in LocStack(rook & self.opp_occ & (self.board.rooks() | self.board.queens()))
        {
            let moves =
                rook & gen_rook_moves(pin, self.opp_occ) | (1 << pin);

            let piece = moves & self.cur_occ;

            if piece.count_ones() == 1 {
                self.pins[piece.trailing_zeros() as usize] = moves;
            }
        }
    }

    fn set_blocks(&mut self) {
        let occ = self.board.occ();

        let king_loc = (self.board.kings() & self.cur_occ).trailing_zeros() as usize;
        self.checks = self.get_threats(king_loc);

        match self.checks.count_ones() {
            0 => {self.blocks = u64::MAX; return},
            1 => {},
            _ => {self.blocks = 0; return},
        }

        let check_loc = self.checks.trailing_zeros() as usize;

        let rook = gen_rook_moves(king_loc, occ);

        if rook & self.checks != 0 {
            self.blocks =
                gen_rook_moves(check_loc, occ) & rook | self.checks;
            return;
        }

        let bishop = gen_bishop_moves(king_loc, occ);

        if bishop & self.checks != 0 {
            self.blocks =
                gen_bishop_moves(check_loc, occ) & bishop | self.checks;
            return;
        }

        self.blocks = self.checks;
    }

    pub fn gen_moves_bits(&mut self) {
        self.moves_bits.clear();

        let occ = self.board.occ();
        let (pawn_shift, pawn_mask1) =
            if self.board.black {
                (Box::new(|x| x >> 8) as Box<dyn Fn(u64) -> u64>,
                    0xffff000000000000)
            } else {
                (Box::new(|x| x << 8) as Box<dyn Fn(u64) -> u64>,
                    0x000000000000ffff)
            };

        let pawn_mask2 = 0x0000ffffffff0000;

        for sq in LocStack(self.board.kings() & self.cur_occ) {
            self.moves_bits.push((sq as u8, TABLES.king[sq] & !self.cur_occ & !self.threatened));
        }

        if self.checks.count_ones() > 1 {
            return;
        }

        for sq in LocStack(self.board.pawns() & self.cur_occ & pawn_mask1) {
            let mut moves;

            moves = pawn_shift(1 << sq) & !occ;
            moves |= pawn_shift(moves);
            moves &= !occ;

            moves |= self.cur_pawn_takes[sq] & (self.opp_occ | self.board.takeable_empties());

            moves &= self.blocks;
            moves &= self.pins[sq];

            self.moves_bits.push((sq as u8, moves));
        }

        for sq in LocStack(self.board.pawns() & self.cur_occ & pawn_mask2) {
            let mut moves = pawn_shift(1 << sq) & !occ;
            moves |= self.cur_pawn_takes[sq] & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            self.moves_bits.push((sq as u8, moves));
        }

        for sq in LocStack(self.board.knights() & self.cur_occ) {
            let mut moves = TABLES.knight[sq] & !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            self.moves_bits.push((sq as u8, moves));
        }

        for sq in LocStack(self.board.bishops() & self.cur_occ) {
            let mut moves = gen_bishop_moves(sq, occ) & !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            self.moves_bits.push((sq as u8, moves));
        }

        for sq in LocStack(self.board.rooks() & self.cur_occ) {
            let mut moves = gen_rook_moves(sq, occ) & !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            self.moves_bits.push((sq as u8, moves));
        }

        for sq in LocStack(self.board.queens() & self.cur_occ) {
            let mut moves = 0;

            moves |= gen_bishop_moves(sq, occ);
            moves |= gen_rook_moves(sq, occ);
            moves &= !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            self.moves_bits.push((sq as u8, moves));
        }

        self.moves_bits.retain(|(_, x)| *x != 0);
    }

    pub fn gen_moves_special(&mut self) {
        let occ = self.board.occ();

        self.moves_special.clear();

        let (pawn_shift, pawn_mask) =
            if self.board.black {
                (Box::new(|x| x >> 8) as Box<dyn Fn(u64) -> u64>,
                    0x000000000000ff00)
            } else {
                (Box::new(|x| x << 8) as Box<dyn Fn(u64) -> u64>,
                    0x00ff000000000000)
            };

        let piece_black = (self.board.black as u8) << 3;

        for sq in LocStack(self.board.pawns() & self.cur_occ & pawn_mask) {
            let mut moves = pawn_shift(1 << sq) & !occ;
            moves |= self.cur_pawn_takes[sq] & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            for sq2 in LocStack(moves) {
                self.moves_special.push(Promotion(piece_black | 4, sq as u8, sq2 as u8));
                self.moves_special.push(Promotion(piece_black | 6, sq as u8, sq2 as u8));
                self.moves_special.push(Promotion(piece_black | 3, sq as u8, sq2 as u8));
                self.moves_special.push(Promotion(piece_black | 2, sq as u8, sq2 as u8));
            }
        }

    }
}

mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn t_get_threats() {
        let mut generator = MoveGenerator::new(Board::from_fen("r7/q7/8/8/8/1nb5/2n5/K1P4r w - -"));

        assert_eq!(generator.checks, 0x0080000000602000);
    }

    #[test]
    fn t_get_threatened() {
        let mut generator = MoveGenerator::new(Board::from_fen("8/b5k1/8/3q1p2/8/1P1n1K2/8/7r w - -"));

        assert_eq!(generator.threatened, 0xd7557fed7f5d47ff);
    }

    #[test]
    fn t_get_blocks() {
        let mut generator = MoveGenerator::empty();

        generator.set_board(Board::from_fen("8/8/8/8/8/2b5/8/K7 w - -")); 
        assert_eq!(generator.blocks, 0x204000);

        generator.set_board(Board::from_fen("8/8/r7/8/8/8/8/K7 w - -")); 
        assert_eq!(generator.blocks, 0x0000808080808000);

        generator.set_board(Board::from_fen("8/8/r7/8/8/2b5/8/K7 w - -")); 
        assert_eq!(generator.blocks, 0);

        generator.set_board(Board::from_fen("8/8/8/8/8/1n6/8/K7 w - -")); 
        assert_eq!(generator.blocks, 0x400000);

        generator.set_board(Board::from_fen("8/8/8/8/8/8/1p6/K7 w - -")); 
        assert_eq!(generator.blocks, 0x4000);

        generator.set_board(Board::from_fen("8/8/8/8/8/2n5/8/K7 w - -")); 
        assert_eq!(generator.blocks, u64::MAX);
    }

    #[test]
    fn t_get_pins() {
        let mut generator = MoveGenerator::new(Board::from_fen("r7/6b1/8/8/8/2P5/P7/KPP4q w - -"));
        let mut res = vec![u64::MAX; 64];

        res[15] = 0x8080808080808000;
        res[21] = 0x0002040810204000;
        generator.set_pins();
        assert_eq!(generator.pins, res);
    }

    #[test]
    fn t_gen_moves_bits() {
        let tables = Tables::new();
        let mut generator = MoveGenerator::empty();
        let mut moves;
        let mut expected;

        generator.set_board(Board::from_fen("8/3Rp1P1/5P2/2B2pK1/2Q5/4N2p/6P1/5P2 w - -"));
        generator.gen_moves_bits();
        moves = generator.moves_bits;
        moves.sort();
        expected = [(2, 0x40400), (9, 0x2030000), (19, 0x1402002010), (29, 0x2048850df70a820), (33, 0x30505000000), (37, 0x88500050800000), (42, 0xc000000000000), (52, 0x10e8101010101010)];
        // print!("[");
        for i in 0..moves.len() {
            // print!("({}, 0x{:x}), ", sq, mov);
            println!("{}", moves[i].0);
            print_board(moves[i].1);
            assert_eq!(moves[i], expected[i]);
        }
        // println!("]");
        // panic!();
    }

    #[bench]
    fn b_get_threats(b: &mut Bencher) {
        let mut generator = MoveGenerator::new(Board::from_fen(START_FEN));
        let king_loc = 4;

        b.iter(|| test::black_box(&generator).get_threats(king_loc));
    }

    #[bench]
    fn b_set_threatened(b: &mut Bencher) {
        let mut generator = MoveGenerator::new(Board::from_fen(START_FEN));

        b.iter(|| {
            test::black_box(&mut generator).set_threatened();
        });
    }

    #[bench]
    fn b_set_pins(b: &mut Bencher) {
        let mut generator = MoveGenerator::new(Board::from_fen(START_FEN));
        // let mut generator = MoveGenerator::new(Board::from_fen("r7/6b1/8/8/8/2P5/P7/KPP4q w - -"));

        b.iter(|| {
            test::black_box(&mut generator).set_pins();
        });
    }

    #[bench]
    fn b_set_blocks(b: &mut Bencher) {
        let mut generator = MoveGenerator::new(Board::from_fen(START_FEN));
        // generator.board = Board::from_fen("r7/6b1/8/8/8/2P5/P7/KPP4q w - -");

        b.iter(|| {
            test::black_box(&mut generator).set_blocks();
        });
    }

    #[bench]
    fn b_set_board(b: &mut Bencher) {
        let mut generator = MoveGenerator::empty();
        let board = Board::from_fen(START_FEN);

        b.iter(|| generator.set_board(test::black_box(board.clone())));
    }
}
