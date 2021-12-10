// this is a submodule of gen_moves so that it can have access to
// MoveGenerator internals
use super::*;
use crate::board::*;
use crate::tt::*;

fn northfill(mut b: u64) -> u64 {
    b |= b << 32;
    b |= b << 16;
    b |= b << 8;

    b
}

fn southfill(mut b: u64) -> u64 {
    b |= b >> 32;
    b |= b >> 16;
    b |= b >> 8;

    b
}

fn adjacent(f: u8) -> u8 {
    f << 1 | f >> 1
}

pub fn w_pawn_threats(pawns: u64) -> u64 {
    (pawns & 0x7f7f7f7f7f7f7f7f) << 9 |
    (pawns & 0xfefefefefefefefe) << 7
}

pub fn b_pawn_threats(pawns: u64) -> u64 {
    (pawns & 0x7f7f7f7f7f7f7f7f) >> 7 |
    (pawns & 0xfefefefefefefefe) >> 9
}

const CHAIN_WEIGHT: i32 = 5;
const PASSED_WEIGHT: i32 = 20;
const DOUBLED_WEIGHT: i32 = -15;
const ISOLATED_WEIGHT: i32 = -15;
const KING_PAWN_WEIGHT: i32 = 10;

const CASTLE_BONUS: i32 = 25;

const KNIGHT_MOVE_WEIGHT: i32 = 10;
const BISHOP_MOVE_WEIGHT: i32 = 10;
const ROOK_MOVE_WEIGHT  : i32 = 5;
const QUEEN_MOVE_WEIGHT : i32 = 2;
const KING_MOVE_WEIGHT  : i32 = 1;

const PAWN_WEIGHT  : i32 = 100;
const KNIGHT_WEIGHT: i32 = 320;
const BISHOP_WEIGHT: i32 = 330;
const ROOK_WEIGHT  : i32 = 500;
const QUEEN_WEIGHT : i32 = 900;
const KING_WEIGHT  : i32 = 25600;

const PIECE_VALUE: [i32; 16] =
    [
        0,
        PAWN_WEIGHT,
        KNIGHT_WEIGHT,
        BISHOP_WEIGHT,
        QUEEN_WEIGHT,
        25600,
        ROOK_WEIGHT,
        ROOK_WEIGHT,
        0,
        -PAWN_WEIGHT,
        -KNIGHT_WEIGHT,
        -BISHOP_WEIGHT,
        -QUEEN_WEIGHT,
        -25600,
        -ROOK_WEIGHT,
        -ROOK_WEIGHT,
    ];

// const CENTER: u64 = 0x00003C3C3C3C0000;
const CENTER: u64 = 0x0000001818000000;
const PAWN_CENTER: u64 = 0x0000003C3C000000;

fn region_bonus(region: u64, moves: u64, weight: i32) -> i32 {
    (moves &  region).count_ones() as i32 * weight * 7 / 5 +
    (moves & !region).count_ones() as i32 * weight
}

pub fn invert_if(b: bool, n: i32) -> i32 {
    if b {
        -n
    } else {
        n
    }
}

impl Board {
    fn eval_material_pawnless(&self) -> i32 {
        let mut out = 0;

        out += self.white_bishops().count_ones() as i32 * BISHOP_WEIGHT;
        out -= self.black_bishops().count_ones() as i32 * BISHOP_WEIGHT;

        out += self.white_knights().count_ones() as i32 * KNIGHT_WEIGHT;
        out -= self.black_knights().count_ones() as i32 * KNIGHT_WEIGHT;

        out += self.white_rooks().count_ones() as i32 * ROOK_WEIGHT;
        out -= self.black_rooks().count_ones() as i32 * ROOK_WEIGHT;

        out += self.white_queens().count_ones() as i32 * QUEEN_WEIGHT;
        out -= self.black_queens().count_ones() as i32 * QUEEN_WEIGHT;

        out
    }

    pub fn eval_material(&self) -> i32 {
        let mut out = 0;

        out += self.white_pawns().count_ones() as i32 * PAWN_WEIGHT;
        out -= self.black_pawns().count_ones() as i32 * PAWN_WEIGHT;

        out += self.white_bishops().count_ones() as i32 * BISHOP_WEIGHT;
        out -= self.black_bishops().count_ones() as i32 * BISHOP_WEIGHT;

        out += self.white_knights().count_ones() as i32 * KNIGHT_WEIGHT;
        out -= self.black_knights().count_ones() as i32 * KNIGHT_WEIGHT;

        out += self.white_rooks().count_ones() as i32 * ROOK_WEIGHT;
        out -= self.black_rooks().count_ones() as i32 * ROOK_WEIGHT;

        out += self.white_queens().count_ones() as i32 * QUEEN_WEIGHT;
        out -= self.black_queens().count_ones() as i32 * QUEEN_WEIGHT;

        out
    }

    pub fn eval_mvv_lva(&self, mov: &Board) -> i32 {
        let mut board = self.clone();
        board.b &= !(self.b ^ mov.b).or();
        board.eval_material()
    }

    fn square_value(&self, black: bool, sq: usize) -> i32 {
        invert_if(black, PIECE_VALUE[self.get_square(sq as u8) as usize])
    }

    fn get_least_valuable(&self, mut att_def: u64, black: bool) -> u64 {
        if black {
            att_def &= self.black();
        } else {
            att_def &= self.white();
        }

        let mut out = 0;

        // if att_def & self.pawns() != 0 {return att_def & self.pawns()}
             if let n@1.. = att_def & self.pawns  () {out = n}
        else if let n@1.. = att_def & self.knights() {out = n}
        else if let n@1.. = att_def & self.bishops() {out = n}
        else if let n@1.. = att_def & self.rooks  () {out = n}
        else if let n@1.. = att_def & self.queens () {out = n}
        else if let n@1.. = att_def & self.kings  () {out = n}

        out & !(out.overflowing_sub(1).0)
    }

    pub fn eval_see(&self, mov: &Board) -> i32 {
        let mov = self.get_move(mov, true);
        let xray = self.pawns() | self.bishops() | self.rooks() | self.queens();

        let to_sq = mov.end();
        let from_sq = mov.start();

        let mut d = 0;
        let mut black = self.black;

        let mut gain = [0; 32];
        let mut occ = self.occ();
        let mut att_def = self.get_att_def(occ, mov.end());
        let mut from = 1u64 << from_sq;

        gain[0] = self.square_value(!black, to_sq);

        while from != 0 {
            d += 1;

            gain[d] = self.square_value(black, from.trailing_zeros() as usize) - gain[d - 1];

            if gain[d].max(-gain[d - 1]) < 0 {break}

            att_def ^= from;
            occ     ^= from;

            if from & xray != 0 {
                att_def = self.get_att_def(occ, mov.end());

            }

            from = self.get_least_valuable(att_def, !black);
            black ^= true;
        }

        d -= 1;

        while d > 0 {
            gain[d - 1] = -gain[d].max(-gain[d - 1]);
            d -= 1;
        }

        return gain[0]
    }

    fn eval_pawns(&self, p_hash: &mut TT) -> i32 {
        if let Some(s) = p_hash.read(self.hash) {
            return s as i32;
        }

        let w = self.white_pawns();
        let b = self.black_pawns();

        let w_threats = w_pawn_threats(w);
        let b_threats = b_pawn_threats(b);

        let w_chains = (w_threats & w).count_ones() as i32;
        let b_chains = (b_threats & b).count_ones() as i32;

        let w_passed = (w &! southfill(b | b_threats)).count_ones() as i32;
        let b_passed = (b &! northfill(w | w_threats)).count_ones() as i32;

        let w_doubled = (northfill(w) << 8 & w).count_ones() as i32;
        let b_doubled = (southfill(b) >> 8 & b).count_ones() as i32;

        let w_files = southfill(w) as u8;
        let b_files = southfill(b) as u8;
        let w_isolated = (w_files &! adjacent(w_files)).count_ones() as i32;
        let b_isolated = (b_files &! adjacent(b_files)).count_ones() as i32;

        let out =
            region_bonus(PAWN_CENTER, w, PAWN_WEIGHT) -
            region_bonus(PAWN_CENTER, b, PAWN_WEIGHT) +
            (w_chains   - b_chains  ) *    CHAIN_WEIGHT +
            (w_passed   - b_passed  ) *   PASSED_WEIGHT +
            (w_doubled  - b_doubled ) *  DOUBLED_WEIGHT +
            (w_isolated - b_isolated) * ISOLATED_WEIGHT;

        p_hash.write(self.hash, out as u64);
        out
    }
}

impl MoveGenerator {
    fn eval_king(&mut self) -> i32 {
        let occ = self.board.occ();

        let kingloc =
            (self.board.kings() & self.cur_occ).trailing_zeros() as usize;

        let mut diag_attacks = gen_bishop_moves(kingloc, occ).count_ones() as i32;
        diag_attacks *=
            ((self.board.queens() | self.board.bishops()) & self.opp_occ)
                .count_ones() as i32;

        let mut rook_attacks = gen_rook_moves(kingloc, occ).count_ones() as i32;

        rook_attacks *=
            ((self.board.queens() | self.board.rooks()) & self.opp_occ)
                .count_ones() as i32;

        let king_pawns =
            (TABLES.king[kingloc] & self.board.pawns() & self.cur_occ)
                .count_ones() as i32;

        let castle_rank = if self.board.black {56} else {0};
        let castle_bonus =
            if kingloc > castle_rank && (kingloc - castle_rank == 1 || kingloc - castle_rank == 5) {
                CASTLE_BONUS
            } else {
                0
            };

        invert_if(
            self.board.black,
            KING_PAWN_WEIGHT * king_pawns +
                castle_bonus -
                diag_attacks -
                rook_attacks
        )
    }

    pub fn eval(&mut self, board: Board, p_hash: &mut TT) -> i32 {
        let occ = board.occ();
        let pawns = board.all_pawns();
        let mut out = pawns.eval_pawns(p_hash);

        for (black, mul) in vec![(true, -1), (false, 1)] {
            self.set_board(Board{ black, .. board });

            out += self.eval_king();

            if !self.has_moves() {
                if self.checks == 0 {
                    return 0;
                } else {
                    return mul * -25600;
                }
            }

            let kingloc =
                (self.board.kings() & self.cur_occ).trailing_zeros() as usize;

            // ========== King Moves ==========
            let moves = TABLES.king[kingloc] & !self.cur_occ & !self.threatened;
            out += mul * KING_MOVE_WEIGHT * moves.count_ones() as i32;

            // ========== Knight Moves ==========
            for sq in LocStack(self.board.knights() & self.cur_occ) {
                let mut moves = TABLES.knight[sq] & !self.cur_occ;

                moves &= self.blocks;
                moves &= self.pins[sq];

                moves &= !self.cur_occ;

                out += region_bonus(CENTER, moves, mul * KNIGHT_MOVE_WEIGHT);
            }

            let cur_diags =
                (self.board.bishops() | self.board.queens()) & self.cur_occ;
            let cur_rook =
                (self.board.rooks() | self.board.queens()) & self.cur_occ;

            // ========== Bishop Moves ==========
            for sq in LocStack(self.board.bishops() & self.cur_occ) {
                let mut moves =
                    gen_bishop_moves(sq, occ & !cur_diags) & !self.cur_occ;

                moves &= self.blocks;
                moves &= self.pins[sq];

                moves &= !self.cur_occ;

                out += region_bonus(CENTER, moves, mul * BISHOP_MOVE_WEIGHT);
            }

            // ========== Rook Moves ==========
            for sq in LocStack(self.board.rooks() & self.cur_occ) {
                let mut moves =
                    gen_rook_moves(sq, occ & !cur_rook) & !self.cur_occ;

                moves &= self.blocks;
                moves &= self.pins[sq];

                moves &= !self.cur_occ;

                out += region_bonus(CENTER, moves, mul * ROOK_MOVE_WEIGHT);
            }

            // ========== Queen Moves ==========
            for sq in LocStack(self.board.queens() & self.cur_occ) {
                let mut moves =
                    gen_rook_moves(sq, occ & !cur_rook) & !self.cur_occ |
                    gen_bishop_moves(sq, occ & !cur_diags) & !self.cur_occ;

                moves &= self.blocks;
                moves &= self.pins[sq];

                moves &= !self.cur_occ;

                out += region_bonus(CENTER, moves, mul * QUEEN_MOVE_WEIGHT);
            }
        }

        invert_if(board.black, out + board.eval_material_pawnless())
    }
}

#[allow(unused_imports)]
use test::Bencher;

#[test]
fn t_eval_pawns() {
    let board = Board::from_fen("8/2pppppp/8/7P/P6P/1P5P/2P5/8 w - -");
    let mut tt = TT::with_len(10);

    assert_eq!(board.eval_pawns(&mut tt), 2 * CHAIN_WEIGHT + 2 * DOUBLED_WEIGHT + ISOLATED_WEIGHT - PASSED_WEIGHT);
}

#[test]
fn t_eval_see() {
    // tests are from https://www.chessprogramming.org/SEE_-_The_Swap_Algorithm
    let board1 = Board::from_fen("1k1r4/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - -");
    let board2 = Board::from_fen("1k1r4/1pp4p/p7/4R3/8/P5P1/1PP4P/2K5 b - -");

    assert_eq!(board1.eval_see(&board2), PAWN_WEIGHT);

    let board1 = Board::from_fen("1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - -");
    let board2 = Board::from_fen("1k1r3q/1ppn3p/p4b2/4N3/8/P5P1/1PP1R1BP/2K1Q3 b - -");

    assert_eq!(board1.eval_see(&board2), PAWN_WEIGHT - KNIGHT_WEIGHT);
}

// #[test]
// fn t_eval_king() {
    // let mut generator = MoveGenerator::new(Board::from_fen("3rrqrr/8/8/8/8/8/5PPP/6K1 w - -"));

    // assert_eq!(generator.eval_king(), KING_PAWN_WEIGHT * 3 - 2 - 40)
// }

#[bench]
fn b_eval_see(b: &mut Bencher) {
    let board1 = Board::from_fen("1k1r4/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - -");
    let board2 = Board::from_fen("1k1r4/1pp4p/p7/4R3/8/P5P1/1PP4P/2K5 b - -");

    b.iter(|| test::black_box(&board1).eval_see(&board2))
}

#[bench]
fn b_eval(b: &mut Bencher) {
    let mut generator = MoveGenerator::empty();
    let board = Board::from_fen("rn1qk2r/p1pnbppp/bp2p3/3pN3/2PP4/1P4P1/P2BPPBP/RN1QK2R w KQkq -");
    let mut tt = TT::with_len(0);

    b.iter(|| generator.eval(board.clone(), &mut tt));
}

#[bench]
fn b_eval_material(b: &mut Bencher) {
    let board = Board::from_fen("rn1qk2r/p1pnbppp/bp2p3/3pN3/2PP4/1P4P1/P2BPPBP/RN1QK2R w KQkq -");

    b.iter(|| test::black_box(&board).eval_material());
}
