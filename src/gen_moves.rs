#[path = "eval.rs"]
pub mod eval;

use crate::gen_tables::*;
use crate::board::*;
use crate::moves::*;

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

#[derive(Debug, PartialEq)]
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
    pub moves: Vec<Board>
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
            moves: Vec::new()
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

    fn get_threats_board(&self, board: &Board, sq: usize) -> u64 {
        let mut out = 0;
        let occ = board.occ();

        out |= self.cur_pawn_takes[sq] & board.pawns();
        out |= TABLES.knight[sq] & board.knights();
        out |= gen_bishop_moves(sq, occ) &
            (board.bishops() | board.queens());
        out |= gen_rook_moves(sq, occ) &
            (board.rooks() | board.queens());

        out & self.opp_occ
    }

    fn get_threats(&self, sq: usize) -> u64 {
        self.get_threats_board(&self.board, sq)
    }

    pub fn get_checks(&self) -> u64 { self.checks }

    fn set_threatened(&mut self) {
        let mut out = 0;

        let mut board = self.board.clone();
        let king = self.board.kings() & self.cur_occ;
        board.b &= !king;

        let occ = board.occ();

        let opp_pawns = board.pawns() & self.opp_occ;

        if board.black {
            out |= eval::w_pawn_threats(opp_pawns);
        } else {
            out |= eval::b_pawn_threats(opp_pawns);
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
        let kingloc = (self.board.kings() & self.cur_occ).trailing_zeros() as usize;

        for p in self.pins.iter_mut() {
            *p = u64::MAX;
        }

        if kingloc == 64 {
            return;
        }

        let bishop = gen_bishop_moves(kingloc, self.opp_occ);
        let rook = gen_rook_moves(kingloc, self.opp_occ);

        for pin in LocStack(bishop & self.opp_occ & (self.board.bishops() | self.board.queens()))
        {
            let moves =
                bishop & gen_bishop_moves(pin, self.opp_occ | self.board.kings()) | (1 << pin);

            let piece = moves & self.cur_occ;

            if piece.count_ones() == 1 {
                self.pins[piece.trailing_zeros() as usize] = moves;
            }
        }

        for pin in LocStack(rook & self.opp_occ & (self.board.rooks() | self.board.queens()))
        {
            let moves =
                rook & gen_rook_moves(pin, self.opp_occ | self.board.kings()) | (1 << pin);

            let piece = moves & self.cur_occ;

            if piece.count_ones() == 1 {
                self.pins[piece.trailing_zeros() as usize] = moves;
            }
        }
    }

    fn set_blocks(&mut self) {
        let occ = self.board.occ();

        let kingloc = (self.board.kings() & self.cur_occ).trailing_zeros() as usize;

        if kingloc == 64 {
            self.checks = 0;
            self.blocks = u64::MAX;
            panic!()
        }

        self.checks = self.get_threats(kingloc);

        match self.checks.count_ones() {
            0 => {self.blocks = u64::MAX; return},
            1 => {},
            _ => {self.blocks = 0; return},
        }

        let check_loc = self.checks.trailing_zeros() as usize;

        let rook = gen_rook_moves(kingloc, occ);

        if rook & self.checks != 0 {
            self.blocks =
                gen_rook_moves(check_loc, occ) & rook | self.checks;
            return;
        }

        let bishop = gen_bishop_moves(kingloc, occ);

        if bishop & self.checks != 0 {
            self.blocks =
                gen_bishop_moves(check_loc, occ) & bishop | self.checks;
            return;
        }

        self.blocks = self.checks;
    }
}

fn do_moves(out: &mut Vec<Board>, board: &Board, sq: usize, moves: u64) {
    let piece = (board.b >> (sq as u32) & 1);

    for sq2 in LocStack(moves) {
        let mut board2 = board.clone();

        board2.b &= !(1 << sq | 1 << sq2);
        board2.b |= piece << (sq2 as u32);
        board2.update_hash(&board);

        out.push(board2);
    }
}

impl MoveGenerator {
    pub fn has_moves(&mut self) -> bool {
        let occ = self.board.occ();

        let kingloc =
            (self.board.kings() & self.cur_occ).trailing_zeros() as usize;

        if kingloc == 64 {
            return false;
        }

        // ========== King Moves ==========
        if TABLES.king[kingloc] & !self.cur_occ & !self.threatened != 0 {
            return true;
        }

        // ========== Pawns ==========
        let pawns = self.board.pawns() & self.cur_occ;

        if self.board.black {
            if eval::b_pawn_threats(pawns) & self.opp_occ != 0 {
                return true;
            } else if pawns >> 8 & !occ != 0 {
                return true;
            }
        } else {
            if eval::w_pawn_threats(pawns) & self.opp_occ != 0 {
                return true;
            } else if pawns << 8 & !occ != 0 {
                return true;
            }
        }


        // ========== Queen Moves ==========
        for sq in LocStack(self.board.queens() & self.cur_occ) {
            let mut moves = 0;

            moves |= gen_bishop_moves(sq, occ);
            moves |= gen_rook_moves(sq, occ);
            moves &= !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            if moves != 0 {
                return true;
            }
        }

        // ========== Bishop Moves ==========
        for sq in LocStack(self.board.bishops() & self.cur_occ) {
            let mut moves = gen_bishop_moves(sq, occ) & !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            if moves != 0 {
                return true;
            }
        }

        // ========== Knight Moves ==========
        for sq in LocStack(self.board.knights() & self.cur_occ) {
            let mut moves = TABLES.knight[sq] & !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            if moves != 0 {
                return true;
            }
        }

        // ========== Rook Moves ==========
        for sq in LocStack(self.board.rooks() & self.cur_occ) {
            let mut moves = gen_rook_moves(sq, occ) & !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            if moves != 0 {
                return true;
            }
        }

        // ========== Castles ==========
        for sq in LocStack(self.board.castling_rooks() & self.cur_occ) {
            let (threat, empty, diff) =
                TABLES.castles[self.board.black as usize][kingloc % 8][sq % 8];

            if occ & empty == 0 && self.threatened & threat == 0 {
                return true;
            }
        }

        // ========== En Passant ==========
        for te in LocStack(self.board.takeable_empties()) {
            if self.opp_pawn_takes[te] & self.board.pawns() & self.cur_occ != 0 {
                return true
            }
        }

        false
    }
    pub fn gen_moves(&mut self) {
        self.moves.clear();

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
        let kingloc =
            (self.board.kings() & self.cur_occ).trailing_zeros() as usize;

        if kingloc == 64 {
            return;
        }

        let mut board = self.board.clone();
        board.black ^= true;
        board.remove_takeable_empty();
        board.update_hash(&self.board);

        // ========== King Moves ==========
        let moves = TABLES.king[kingloc] & !self.cur_occ & !self.threatened;
        let mut board2 = board.clone();
        board2.b ^= u64x4::new(0,0,0,board.castling_rooks() & self.cur_occ);
        board2.update_hash(&board);

        do_moves(&mut self.moves, &board2, kingloc, moves);

        if self.checks.count_ones() > 1 {
            return;
        }

        // ========== Castles ==========
        for sq in LocStack(self.board.castling_rooks() & self.cur_occ) {
            let (threat, empty, diff) =
                TABLES.castles[self.board.black as usize][kingloc % 8][sq % 8];

            if occ & empty == 0 && self.threatened & threat == 0 {
                let mut board2 = board.clone();
                board2.b ^= diff;
                board2.b ^= u64x4::new(0,0,0,board2.castling_rooks() & self.cur_occ);
                board2.update_hash(&board);

                self.moves.push(board2);
            }
        }

        // ========== Double-Moving Pawns ==========
        for sq in LocStack(self.board.pawns() & self.cur_occ & pawn_mask1) {
            let mut moves;
            let mut ep_moves;

            moves = pawn_shift(1 << sq) & !occ;
            ep_moves = pawn_shift(moves) & !occ;
            moves &= !occ;

            moves |= self.cur_pawn_takes[sq] & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            ep_moves &= self.blocks;
            ep_moves &= self.pins[sq];

            do_moves(&mut self.moves, &board, sq, moves);

            for sq2 in LocStack(ep_moves) {
                let mut board2 = board.clone();
                board2.b |= u64x4::new(pawn_shift(1 << sq),0,0,0);
                board2.b &= !(1 << sq);
                board2.b |= u64x4::new(self.board.black as u64,0,0,1)
                    << (sq2 as u32);
                board2.update_hash(&board);

                self.moves.push(board2);
            }
        }

        // ========== Other Non-Promoting Pawns ==========
        for sq in LocStack(self.board.pawns() & self.cur_occ & pawn_mask2) {
            let mut moves = pawn_shift(1 << sq) & !occ;
            moves |= self.cur_pawn_takes[sq] & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            do_moves(&mut self.moves, &board, sq, moves);
        }

        // ========== Promoting Pawns ==========
        for sq in LocStack(self.board.pawns() & self.cur_occ & !(pawn_mask1 | pawn_mask2)) {
            let mut moves = pawn_shift(1 << sq) & !occ;
            moves |= self.cur_pawn_takes[sq] & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            let mut board2 = board.clone();
            board2.b &= !(1 << sq);
            board2.update_hash(&board);

            for sq2 in LocStack(moves) {
                let sq2 = sq2 as u32;
                let mut board3 = board2.clone();
                board3.b &= !(1 << sq2);

                board3.b ^= u64x4::new(self.board.black as u64, 1, 0, 0) << sq2;
                board3.update_hash(&board2);
                self.moves.push(board3.clone());

                board3.b ^= u64x4::new(0, 0, 1, 0) << sq2;
                board3.update_hash(&board2);
                self.moves.push(board3.clone());

                board3.b ^= u64x4::new(0, 1, 0, 1) << sq2;
                board3.update_hash(&board2);
                self.moves.push(board3.clone());

                board3.b ^= u64x4::new(0, 0, 0, 1) << sq2;
                board3.update_hash(&board2);
                self.moves.push(board3.clone());
            }
        }

        // ========== En Passant ==========
        for te in LocStack(self.board.takeable_empties()) {
            for sq in LocStack(self.opp_pawn_takes[te] & self.board.pawns() & self.cur_occ) {
                let mut board2 = board.clone();
                board2.b ^= TABLES.en_pass
                    [self.board.black as usize]
                    [(te % 8 > sq % 8) as usize] << sq as u32 % 8;
                board2.update_hash(&board);
                board2.black ^= true;

                if self.get_threats_board(&board2, kingloc) == 0 {
                    board2.black ^= true;
                    self.moves.push(board2);
                }
            }
        }

        // ========== Knight Moves ==========
        for sq in LocStack(self.board.knights() & self.cur_occ) {
            let mut moves = TABLES.knight[sq] & !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            do_moves(&mut self.moves, &board, sq, moves);
        }

        // ========== Bishop Moves ==========
        for sq in LocStack(self.board.bishops() & self.cur_occ) {
            let mut moves = gen_bishop_moves(sq, occ) & !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            do_moves(&mut self.moves, &board, sq, moves);
        }

        // ========== Rook Moves ==========
        for sq in LocStack(self.board.rooks() & self.cur_occ) {
            let mut moves = gen_rook_moves(sq, occ) & !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            let mut board2 = board.clone();
            board2.b &= u64x4::new(M,M,M,!(1 << sq));
            board2.update_hash(&board);

            do_moves(&mut self.moves, &board2, sq, moves);
        }

        // ========== Queen Moves ==========
        for sq in LocStack(self.board.queens() & self.cur_occ) {
            let mut moves = 0;

            moves |= gen_bishop_moves(sq, occ);
            moves |= gen_rook_moves(sq, occ);
            moves &= !self.cur_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            do_moves(&mut self.moves, &board, sq, moves);
        }
    }

    pub fn gen_tactical(&mut self) {
        self.moves.clear();

        let occ = self.board.occ();
        let (pawn_shift, promote_mask) =
            if self.board.black {
                (Box::new(|x| x >> 8) as Box<dyn Fn(u64) -> u64>,
                    0x000000000000ffff)
            } else {
                (Box::new(|x| x << 8) as Box<dyn Fn(u64) -> u64>,
                    0xffff000000000000)
            };

        let kingloc =
            (self.board.kings() & self.cur_occ).trailing_zeros() as usize;

        if kingloc == 64 {
            return;
        }

        let mut board = self.board.clone();
        board.black ^= true;
        board.remove_takeable_empty();
        board.update_hash(&self.board);

        // ========== King Takes ==========
        let moves = TABLES.king[kingloc] & self.opp_occ & !self.threatened;
        let mut board2 = board.clone();
        board2.b ^= u64x4::new(0,0,0,board.castling_rooks() & self.cur_occ);
        board2.update_hash(&board);

        do_moves(&mut self.moves, &board2, kingloc, moves);

        if self.checks.count_ones() > 1 {
            return;
        }

        // ========== Non-Promoting Pawn Takes ==========
        for sq in LocStack(self.board.pawns() & self.cur_occ & !promote_mask) {
            let mut moves = self.cur_pawn_takes[sq] & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            do_moves(&mut self.moves, &board, sq, moves);
        }

        // ========== Promoting Pawn Moves ==========
        for sq in LocStack(self.board.pawns() & self.cur_occ & promote_mask) {
            let mut moves = pawn_shift(1 << sq) & !occ;
            moves |= self.cur_pawn_takes[sq] & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            let mut board2 = board.clone();
            board2.b &= !(1 << sq);
            board2.update_hash(&board);

            for sq2 in LocStack(moves) {
                let sq2 = sq2 as u32;
                let mut board3 = board2.clone();
                board3.b &= !(1 << sq2);

                board3.b ^= u64x4::new(self.board.black as u64, 1, 0, 0) << sq2;
                board3.update_hash(&board2);
                self.moves.push(board3.clone());

                board3.b ^= u64x4::new(0, 0, 1, 0) << sq2;
                board3.update_hash(&board2);
                self.moves.push(board3.clone());

                board3.b ^= u64x4::new(0, 1, 0, 1) << sq2;
                board3.update_hash(&board2);
                self.moves.push(board3.clone());

                board3.b ^= u64x4::new(0, 0, 0, 1) << sq2;
                board3.update_hash(&board2);
                self.moves.push(board3.clone());
            }
        }

        // ========== En Passant ==========
        for te in LocStack(self.board.takeable_empties()) {
            for sq in LocStack(self.opp_pawn_takes[te] & self.board.pawns() & self.cur_occ) {
                let mut board2 = board.clone();
                board2.b ^= TABLES.en_pass
                    [self.board.black as usize]
                    [(te % 8 > sq % 8) as usize] << sq as u32 % 8;
                board2.update_hash(&board);
                board2.black ^= true;

                if self.get_threats_board(&board2, kingloc) == 0 {
                    board2.black ^= true;
                    self.moves.push(board2);
                }
            }
        }

        // ========== Knight Takes ==========
        for sq in LocStack(self.board.knights() & self.cur_occ) {
            let mut moves = TABLES.knight[sq] & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            do_moves(&mut self.moves, &board, sq, moves);
        }

        // ========== Bishop Takes ==========
        for sq in LocStack(self.board.bishops() & self.cur_occ) {
            let mut moves = gen_bishop_moves(sq, occ) & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            do_moves(&mut self.moves, &board, sq, moves);
        }

        // ========== Rook Takes ==========
        for sq in LocStack(self.board.rooks() & self.cur_occ) {
            let mut moves = gen_rook_moves(sq, occ) & self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            let mut board2 = board.clone();
            board2.b &= u64x4::new(M,M,M,!(1 << sq));
            board2.update_hash(&board);

            do_moves(&mut self.moves, &board2, sq, moves);
        }

        // ========== Queen Takes ==========
        for sq in LocStack(self.board.queens() & self.cur_occ) {
            let mut moves = 0;

            moves |= gen_bishop_moves(sq, occ);
            moves |= gen_rook_moves(sq, occ);
            moves &= self.opp_occ;

            moves &= self.blocks;
            moves &= self.pins[sq];

            do_moves(&mut self.moves, &board, sq, moves);
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

    fn gen_t_gen_moves(board: Board, mut moves2: Vec<Board>, full: bool) {
        let mut generator = MoveGenerator::new(board);

        generator.gen_moves();

        let moves = &mut generator.moves;

        moves.sort_by_key(|b| b.hash);
        moves2.sort_by_key(|b| b.hash);

        if !full {
            println!("{:?}", moves);
        }

        for mov2 in moves2.iter() {
            if !moves.contains(mov2) {
                println!("'moves' does not contain");
                println!("{:?}", mov2);
                panic!()
            }
        }
        if full {
            for mov1 in moves.iter() {
                if !moves2.contains(mov1) {
                    println!("'moves2' does not contain");
                    println!("{:?}", mov1);
                    panic!()
                }
            }
            assert_eq!(moves, &moves2);
        }
    }

    #[test]
    fn t_gen_moves() {
        let mut expected;
        let mut board = Board::from_fen("8/3Rp1P1/5P2/2B2pK1/2Q5/4N2p/8/8 w - -");

        let mut moves2 = Vec::new();

        expected = vec![(19, 0x1402002214), (29, 0x2048850df70a824), (33, 0x30505000000), (37, 0x88500050800000), (42, 0xc000000000000), (52, 0x10e8101010101010)];

        let mut board2 = board.clone();
        board2.black ^= true;
        board2.remove_takeable_empty();
        board2.update_hash(&board);

        for (sq, moves) in expected {
            do_moves(&mut moves2, &board2, sq, moves);
        }

        moves2.push(Board::from_fen("6Q1/3Rp3/5P2/2B2pK1/2Q5/4N2p/8/8 b - -"));
        moves2.push(Board::from_fen("6R1/3Rp3/5P2/2B2pK1/2Q5/4N2p/8/8 b - -"));
        moves2.push(Board::from_fen("6B1/3Rp3/5P2/2B2pK1/2Q5/4N2p/8/8 b - -"));
        moves2.push(Board::from_fen("6N1/3Rp3/5P2/2B2pK1/2Q5/4N2p/8/8 b - -"));

        gen_t_gen_moves(board, moves2, true);

        board = Board::from_fen("PPPPPPK1/PPPPPPP1/8/4pP2/8/8/8/8 w - e6");
        moves2 = vec![
            Board::from_fen("PPPPPP1K/PPPPPPP1/8/4pP2/8/8/8/8 b - -"),
            Board::from_fen("PPPPPPK1/PPPPPPP1/5P2/4p3/8/8/8/8 b - -"),
            Board::from_fen("PPPPPP2/PPPPPPPK/8/4pP2/8/8/8/8 b - -"),
            Board::from_fen("PPPPPPK1/PPPPPPP1/4P3/8/8/8/8/8 b - -"),
        ];

        gen_t_gen_moves(board, moves2, true);

        board = Board::from_fen("8/8/8/8/8/8/8/R3K2R w KQ -");
        moves2 = vec![
            Board::from_fen("8/8/8/8/8/8/8/R3K1R1 b Q -"),
            Board::from_fen("8/8/8/8/8/8/8/1R2K2R b K -"),
            Board::from_fen("8/8/8/8/8/8/8/R4RK1 b - -"),
            Board::from_fen("8/8/8/8/8/8/8/2KR3R b - -"),
            Board::from_fen("8/8/8/8/8/8/8/R2K3R b - -"),
        ];

        gen_t_gen_moves(board, moves2, false);

        board = Board::from_fen("K4p1p/3P2P1/8/8/8/8/8/8 w - -");

        moves2 = vec![
            Board::from_fen("K2Q1p1p/6P1/8/8/8/8/8/8 b - -"),
            Board::from_fen("K4R1p/3P4/8/8/8/8/8/8 b - -"),
            Board::from_fen("K4pBp/3P4/8/8/8/8/8/8 b - -"),
            Board::from_fen("K4p1N/3P4/8/8/8/8/8/8 b - -"),
        ];

        gen_t_gen_moves(board, moves2, false);
    }

    #[bench]
    fn b_gen_moves(b: &mut Bencher) {
        let mut generator = MoveGenerator::new(Board::from_fen("8/3Rp1P1/5P2/2B2pK1/2Q5/4N2p/6P1/5P2 w - -"));
        // let mut generator = MoveGenerator::new(Board::from_fen(START_FEN));

        b.iter(|| generator.gen_moves());
    }

    #[bench]
    fn b_gen_tactical(b: &mut Bencher) {
        let mut generator = MoveGenerator::new(Board::from_fen("8/3Rp1P1/5P2/2B2pK1/2Q5/4N2p/6P1/5P2 w - -"));
        // let mut generator = MoveGenerator::new(Board::from_fen(START_FEN));

        b.iter(|| generator.gen_tactical());
    }

    #[bench]
    fn b_get_threats(b: &mut Bencher) {
        let mut generator = MoveGenerator::new(Board::from_fen(START_FEN));
        let kingloc = 3;

        b.iter(|| test::black_box(&generator).get_threats(kingloc));
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
