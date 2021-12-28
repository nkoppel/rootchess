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

pub const CHECKMATE: i32 = 25600;

#[derive(Clone, Debug)]
pub struct EvalParams {
    pub chain_weight: i32,
    pub passed_weight: i32,
    pub doubled_weight: i32,
    pub isolated_weight: i32,
    pub king_pawn_weight: i32,

    pub castle_bonus: i32,

    pub knight_move_weight: i32,
    pub bishop_move_weight: i32,
    pub rook_move_weight  : i32,
    pub queen_move_weight : i32,
    pub king_move_weight  : i32,

    pub pawn_weight  : i32,
    pub knight_weight: i32,
    pub bishop_weight: i32,
    pub rook_weight  : i32,
    pub queen_weight : i32,
    pub king_weight  : i32,

    pub psts: [[i32; 64]; 16],
}

impl Default for EvalParams {
    fn default() -> Self {
        Self {
            chain_weight: -2,
            passed_weight: -6,
            doubled_weight: -5,
            isolated_weight: 2,
            king_pawn_weight: -5,
            castle_bonus: -49,
            knight_move_weight: 0,
            bishop_move_weight: 1,
            rook_move_weight: 2,
            queen_move_weight: 3,
            king_move_weight: 1,
            pawn_weight: 0,
            knight_weight: 0,
            bishop_weight: 0,
            rook_weight: 0,
            queen_weight: 0,
            king_weight: 25600,
            psts: [
                [
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                ],
                [
                    0, 0, 0, 0, 0, 0, 0, 0,
                    32, 27, 28, 19, 22, 28, 32, 26,
                    29, 23, 32, 27, 29, 33, 29, 24,
                    23, 29, 17, 25, 27, 21, 31, 26,
                    40, 36, 30, 31, 28, 28, 50, 46,
                    78, 57, 55, 70, 74, 73, 70, 63,
                    94, 121, 97, 83, 83, 134, 114, 94,
                    0, 0, 0, 0, 0, 0, 0, 0,
                ],
                [
                    34, 16, 32, 35, 42, 19, 28, 28,
                    52, 50, 39, 36, 31, 44, 49, -5,
                    29, 49, 41, 51, 45, 39, 42, 20,
                    40, 51, 49, 47, 41, 41, 48, 40,
                    62, 41, 66, 51, 65, 50, 45, 41,
                    35, 42, 47, 61, 60, 45, 54, 51,
                    43, 75, 50, 48, 48, 64, 45, 29,
                    23, 76, 51, 52, 52, 56, 38, 68,
                ],
                [
                    46, -14, 30, 22, 30, 35, 36, 59,
                    60, 48, 47, 43, 40, 45, 40, 21,
                    37, 57, 44, 42, 43, 43, 41, 31,
                    32, 30, 39, 46, 42, 39, 37, 48,
                    42, 39, 35, 49, 50, 48, 41, 29,
                    58, 44, 41, 40, 44, 39, 50, 36,
                    7, 43, 29, 39, 47, 48, 44, 50,
                    97, 58, 36, 52, 36, 62, 37, 58,
                ],
                [
                    39, 103, 60, 86, 87, 83, 60, 75,
                    61, 84, 90, 89, 86, 91, 67, 98,
                    119, 91, 87, 82, 84, 84, 84, 78,
                    86, 90, 89, 85, 86, 94, 84, 85,
                    90, 78, 108, 97, 78, 87, 79, 92,
                    108, 118, 110, 114, 97, 113, 92, 98,
                    109, 131, 133, 107, 110, 114, 102, 99,
                    134, 143, 132, 114, 76, 132, 108, 125,
                ],
                [
                    -99, -41, -100, -89, -99, -39, -87, -122,
                    -92, -89, -86, -91, -91, -91, -86, -98,
                    -92, -88, -78, -80, -81, -83, -85, -89,
                    -108, -82, -82, -75, -69, -76, -82, -101,
                    -87, -79, -70, -78, -75, -74, -77, -74,
                    -103, -88, -75, -72, -69, -81, -79, -70,
                    -112, -102, -104, -78, -94, -97, -85, -35,
                    -38, -99, -84, -115, -95, -81, -53, -34,
                ],
                [
                    29, 51, 44, 47, 47, 45, 43, 40,
                    34, 44, 48, 50, 50, 51, 45, 49,
                    48, 33, 47, 46, 43, 48, 41, 41,
                    51, 46, 53, 51, 52, 56, 55, 54,
                    53, 45, 60, 47, 50, 55, 52, 57,
                    57, 60, 57, 59, 57, 58, 62, 60,
                    54, 58, 61, 61, 63, 61, 61, 56,
                    52, 63, 52, 51, 58, 58, 60, 60,
                ],
                [
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                ],
                [
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                ],
                [
                    0, 0, 0, 0, 0, 0, 0, 0,
                    82, 88, 110, 84, 101, 102, 108, 104,
                    60, 71, 54, 56, 64, 74, 74, 68,
                    31, 43, 30, 34, 34, 33, 39, 33,
                    17, 30, 19, 29, 29, 26, 28, 31,
                    24, 28, 30, 31, 23, 27, 22, 30,
                    23, 33, 29, 27, 15, 28, 26, 25,
                    0, 0, 0, 0, 0, 0, 0, 0,
                ],
                [
                    28, 65, 60, 57, 51, 73, 42, 45,
                    33, 45, 51, 49, 60, 61, 36, 37,
                    44, 79, 40, 50, 54, 46, 43, 44,
                    43, 40, 61, 53, 52, 49, 45, 33,
                    56, 34, 49, 43, 51, 51, 48, 38,
                    38, 42, 45, 40, 44, 38, 33, 23,
                    23, 30, 46, 39, 34, 26, 34, 19,
                    -29, 23, 9, 13, 31, 37, 29, 4,
                ],
                [
                    42, 48, 42, 43, 37, 50, 35, 57,
                    4, 34, 50, 37, 29, 34, 42, 37,
                    34, 43, 33, 46, 42, 37, 48, 39,
                    45, 27, 40, 45, 52, 39, 40, 20,
                    48, 26, 35, 39, 38, 44, 38, 32,
                    27, 51, 42, 35, 38, 40, 33, 26,
                    86, 43, 48, 42, 31, 30, 37, 49,
                    28, 27, 25, 26, 31, 27, 80, 31,
                ],
                [
                    143, 136, 128, 112, 88, 117, 122, 116,
                    112, 104, 113, 94, 96, 91, 81, 93,
                    114, 107, 106, 109, 105, 98, 85, 99,
                    101, 87, 99, 92, 80, 88, 69, 91,
                    83, 93, 81, 79, 69, 84, 79, 76,
                    90, 91, 82, 82, 77, 87, 81, 82,
                    96, 86, 84, 82, 90, 88, 86, 104,
                    93, 30, 68, 86, 87, 89, 91, 78,
                ],
                [
                    -80, -63, -72, -58, -87, -62, -35, -22,
                    -82, -111, -100, -70, -73, -91, -90, -70,
                    -93, -84, -75, -70, -60, -79, -74, -82,
                    -89, -81, -76, -74, -68, -63, -73, -67,
                    -95, -85, -78, -81, -74, -72, -80, -86,
                    -91, -84, -82, -83, -87, -78, -77, -89,
                    -88, -85, -87, -88, -93, -86, -77, -88,
                    -93, -35, -97, -84, -89, -38, -88, -87,
                ],
                [
                    53, 60, 50, 48, 53, 60, 55, 56,
                    49, 59, 66, 60, 58, 59, 61, 59,
                    60, 56, 60, 55, 51, 57, 58, 56,
                    54, 56, 56, 51, 49, 53, 54, 59,
                    49, 53, 54, 50, 50, 58, 58, 51,
                    50, 53, 54, 49, 48, 48, 47, 52,
                    44, 48, 56, 51, 48, 51, 46, 50,
                    32, 43, 47, 51, 48, 49, 49, 43,
                ],
                [
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0,
                ],
            ],
        }
    }
}

lazy_static! {
    pub static ref PARAMS: EvalParams = EvalParams::default();
    static ref PIECE_VALUE: [i32; 16] =
        [
            0,
            PARAMS.pawn_weight,
            PARAMS.knight_weight,
            PARAMS.bishop_weight,
            PARAMS.queen_weight,
            PARAMS.king_weight,
            PARAMS.rook_weight,
            PARAMS.rook_weight,
            0,
            -PARAMS.pawn_weight,
            -PARAMS.knight_weight,
            -PARAMS.bishop_weight,
            -PARAMS.queen_weight,
            -PARAMS.king_weight,
            -PARAMS.rook_weight,
            -PARAMS.rook_weight,
        ];
}

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
    fn eval_material_pawnless(&self, params: &EvalParams) -> i32 {
        let mut out = 0;

        out += self.white_bishops().count_ones() as i32 * params.bishop_weight;
        out -= self.black_bishops().count_ones() as i32 * params.bishop_weight;

        out += self.white_knights().count_ones() as i32 * params.knight_weight;
        out -= self.black_knights().count_ones() as i32 * params.knight_weight;

        out += self.white_rooks().count_ones() as i32 * params.rook_weight;
        out -= self.black_rooks().count_ones() as i32 * params.rook_weight;

        out += self.white_queens().count_ones() as i32 * params.queen_weight;
        out -= self.black_queens().count_ones() as i32 * params.queen_weight;

        out
    }

    pub fn eval_material(&self, params: &EvalParams) -> i32 {
        let mut out = 0;

        out += self.white_pawns().count_ones() as i32 * params.pawn_weight;
        out -= self.black_pawns().count_ones() as i32 * params.pawn_weight;

        out += self.white_bishops().count_ones() as i32 * params.bishop_weight;
        out -= self.black_bishops().count_ones() as i32 * params.bishop_weight;

        out += self.white_knights().count_ones() as i32 * params.knight_weight;
        out -= self.black_knights().count_ones() as i32 * params.knight_weight;

        out += self.white_rooks().count_ones() as i32 * params.rook_weight;
        out -= self.black_rooks().count_ones() as i32 * params.rook_weight;

        out += self.white_queens().count_ones() as i32 * params.queen_weight;
        out -= self.black_queens().count_ones() as i32 * params.queen_weight;

        out
    }

    pub fn eval_mvv_lva(&self, mov: &Board) -> i32 {
        let mut board = self.clone();
        board.b &= !(self.b ^ mov.b).or();
        board.eval_material(&PARAMS)
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

    fn eval_pawns(&self, p_hash: &mut TT, params: &EvalParams) -> i32 {
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

        let mut out = (w.count_ones() as i32 - b.count_ones() as i32) * params.pawn_weight;

        out += (w_chains   - b_chains  ) *    params.chain_weight;
        out += (w_passed   - b_passed  ) *   params.passed_weight;
        out += (w_doubled  - b_doubled ) *  params.doubled_weight;
        out += (w_isolated - b_isolated) * params.isolated_weight;

        for sq in LocStack(w) {
            out += params.psts[1][sq];
        }

        for sq in LocStack(b) {
            out -= params.psts[9][sq];
        }

        p_hash.write(self.hash, out as u64);
        out
    }
}

impl MoveGenerator {
    fn eval_king(&mut self, params: &EvalParams) -> i32 {
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
                params.castle_bonus
            } else {
                0
            };

        invert_if(
            self.board.black,
            params.king_pawn_weight * king_pawns +
                castle_bonus -
                diag_attacks -
                rook_attacks
        )
    }

    pub fn eval_with_params(&mut self, board: Board, p_hash: &mut TT, params: &EvalParams) -> i32 {
        let occ = board.occ();
        let pawns = board.all_pawns();
        let mut out = pawns.eval_pawns(p_hash, params);

        for (black, mul) in vec![(true, -1), (false, 1)] {
            self.set_board(Board{ black, .. board });

            out += self.eval_king(params);

            if board.black == black && !self.has_moves() {
                if self.checks == 0 {
                    return 0;
                } else {
                    return mul * -CHECKMATE;
                }
            }

            let kingloc =
                (self.board.kings() & self.cur_occ).trailing_zeros() as usize;

            // ========== King Moves ==========
            let moves = TABLES.king[kingloc] & !self.cur_occ & !self.threatened;
            out += mul * params.king_move_weight * moves.count_ones() as i32;
            out += mul * params.psts[(black as usize) << 3 | 5][kingloc];

            // ========== Knight Moves ==========
            for sq in LocStack(self.board.knights() & self.cur_occ) {
                let mut moves = TABLES.knight[sq] & !self.cur_occ;

                moves &= self.blocks;
                moves &= self.pins[sq];

                moves &= !self.cur_occ;

                out += mul * moves.count_ones() as i32 * params.knight_move_weight;
                out += mul * params.psts[(black as usize) << 3 | 2][sq];
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

                out += mul * moves.count_ones() as i32 * params.bishop_move_weight;
                out += mul * params.psts[(black as usize) << 3 | 3][sq];
            }

            // ========== Rook Moves ==========
            for sq in LocStack(self.board.rooks() & self.cur_occ) {
                let mut moves =
                    gen_rook_moves(sq, occ & !cur_rook) & !self.cur_occ;

                moves &= self.blocks;
                moves &= self.pins[sq];

                moves &= !self.cur_occ;

                out += mul * moves.count_ones() as i32 * params.rook_move_weight;
                out += mul * params.psts[(black as usize) << 3 | 6][sq];
            }

            // ========== Queen Moves ==========
            for sq in LocStack(self.board.queens() & self.cur_occ) {
                let mut moves =
                    gen_rook_moves(sq, occ & !cur_rook) & !self.cur_occ |
                    gen_bishop_moves(sq, occ & !cur_diags) & !self.cur_occ;

                moves &= self.blocks;
                moves &= self.pins[sq];

                moves &= !self.cur_occ;

                out += mul * moves.count_ones() as i32 * params.queen_move_weight;
                out += mul * params.psts[(black as usize) << 3 | 4][sq];
            }
        }

        invert_if(board.black, out + board.eval_material_pawnless(params))
    }

    pub fn eval(&mut self, board: Board, p_hash: &mut TT) -> i32 {
        self.eval_with_params(board, p_hash, &PARAMS)
    }
}

#[allow(unused_imports)]
use test::Bencher;

#[test]
fn t_eval_pawns() {
    let board = Board::from_fen("8/2pppppp/8/7P/P6P/1P5P/2P5/8 w - -");
    let mut tt = TT::with_len(10);

    assert_eq!(board.eval_pawns(&mut tt, &PARAMS), 2 * PARAMS.chain_weight + 2 * PARAMS.doubled_weight + PARAMS.isolated_weight - PARAMS.passed_weight);
}

#[test]
fn t_eval_see() {
    // tests are from https://www.chessprogramming.org/SEE_-_The_Swap_Algorithm
    let board1 = Board::from_fen("1k1r4/1pp4p/p7/4p3/8/P5P1/1PP4P/2K1R3 w - -");
    let board2 = Board::from_fen("1k1r4/1pp4p/p7/4R3/8/P5P1/1PP4P/2K5 b - -");

    assert_eq!(board1.eval_see(&board2), PARAMS.pawn_weight);

    let board1 = Board::from_fen("1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - -");
    let board2 = Board::from_fen("1k1r3q/1ppn3p/p4b2/4N3/8/P5P1/1PP1R1BP/2K1Q3 b - -");

    assert_eq!(board1.eval_see(&board2), PARAMS.pawn_weight - PARAMS.knight_weight);
}

// #[test]
// fn t_eval_king() {
    // let mut generator = MoveGenerator::new(Board::from_fen("3rrqrr/8/8/8/8/8/5PPP/6K1 w - -"));

    // assert_eq!(generator.eval_king(), PARAMS.king_pawn_weight * 3 - 2 - 40)
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

    b.iter(|| test::black_box(&board).eval_material(&PARAMS));
}
