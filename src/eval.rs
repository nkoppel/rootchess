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
        EvalParams {
            chain_weight: 11,
            passed_weight: 41,
            doubled_weight: -9,
            isolated_weight: -13,
            king_pawn_weight: 7,

            castle_bonus: -4,

            knight_move_weight: 5,
            bishop_move_weight: 6,
            rook_move_weight: 3,
            queen_move_weight: 1,
            king_move_weight: -5,

            pawn_weight: 100,
            knight_weight: 279,
            bishop_weight: 293,
            rook_weight: 466,
            queen_weight: 866,
            king_weight: 25600,

            psts: [
                [ // Empty
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                ],
                [ // White Pawn
                       0,    0,    0,    0,    0,    0,    0,    0,
                      -8,   -5,   -2,  -23,  -27,    2,   -2,   -2,
                       4,    2,  -11,   -7,  -21,    0,   -4,   -5,
                       0,    5,   -2,    8,    9,    1,   10,    3,
                       5,   13,   15,   17,   26,   18,   28,   19,
                      39,   54,   47,   42,   42,   57,   61,   77,
                      89,  107,   84,   84,   93,   97,  137,   89,
                       0,    0,    0,    0,    0,    0,    0,    0,
                ],
                [ // White Knight
                      29,  -19,  -26,   -7,  -20,   -8,  -14,  -33,
                      -3,    6,   -6,   14,    7,    6,   38,  -15,
                      -9,   28,   16,   21,    8,    8,    6,  -18,
                      -5,   31,   24,   24,   13,   10,   10,  -14,
                      61,   24,   20,   47,   38,   26,   22,   15,
                      20,   51,   33,   55,   38,   42,   41,  -16,
                      -7,   24,   48,    1,   20,   31,    2,  -42,
                     -35,   34,   13,   16,   60,   -7,   46,   -6,
                ],
                [ // White bishop
                      37,   37,  -17,   10,  -19,    5,   -9,   63,
                      44,   31,    7,   14,   16,   13,   40,   65,
                       3,   20,   16,   19,   15,   21,   32,   35,
                      -8,    8,   32,    7,    9,   13,   17,   31,
                     -26,    9,   20,   21,   17,   22,   -1,   18,
                      40,   18,   23,   17,   48,   16,   33,    7,
                     -12,    5,    6,   34,   14,   11,   32,    3,
                      16,   -3,   13,   17,   28,    8,   33,   41,
                ],
                [ // White Queen
                      39,  -82,   -7,  -11,  -11,  -26,  -26,   45,
                     -63,   -4,  -23,    2,  -11,    0,    5,   12,
                       1,   -7,    7,  -13,   -9,   10,   -3,  -27,
                      15,   25,   -5,  -18,    4,  -16,   -8,  -15,
                      10,   20,   34,   18,   13,   -6,  -45,    3,
                      37,   66,   70,   34,   48,  -25,   27,  -19,
                      59,   64,   50,   64,   10,   19,  -30,  -19,
                      20,   -3,   51,   46,   23,   51,   17,   11,
                ],
                [ // White King
                     -50,   -8,  -28,   -6,  -45,   -7,  -13,   12,
                     -23,   -4,    9,    7,    4,    1,    4,   22,
                     -29,    6,   13,   14,   18,    9,   17,   20,
                       0,   27,   35,   39,   47,   37,   35,   -2,
                      26,   48,   61,   70,   68,   75,   58,   28,
                      45,   64,   85,   74,   78,   80,   95,   91,
                      94,   58,   72,   90,   91,   71,   93,  455,
                     -22,   71,   51,  142,  192,  170,  548,  588,
                ],
                [ // White Rook
                     -15,    7,   21,   19,   21,   22,   22,   -5,
                     -12,  -10,   17,   15,   16,   13,   -8,   -8,
                     -12,   30,   11,   16,   22,   -1,    3,    5,
                      12,    3,   25,   10,   14,   28,   16,   13,
                      30,    6,   40,   33,   43,   28,   26,   24,
                      20,   31,   29,   37,   42,   35,   32,   25,
                      24,   39,   48,   56,   62,   35,   43,   31,
                      27,   25,   43,   44,   39,   26,   33,   34,
                ],
                [ // White Uncastled Rook
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                ],
                [ // Empty
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                ],
                [ // Black Pawn
                       0,    0,    0,    0,    0,    0,    0,    0,
                      66,  115,  115,  113,   95,  122,  142,   87,
                      47,   75,   45,   46,   29,   78,   90,   79,
                      28,   30,    9,   17,   18,   13,   38,   32,
                      -2,    6,   -3,    5,    7,    6,   13,    4,
                      -9,    2,   -2,  -15,  -11,   -5,   -8,   -1,
                     -14,    3,    5,  -31,  -32,    1,    5,   -7,
                       0,    0,    0,    0,    0,    0,    0,    0,
                ],
                [ // Black Knight
                     -11,   32,  -32,   20,    4,   40,    6,  -15,
                       8,  -34,   49,   25,   37,   36,    1,  -37,
                      23,   31,   29,   36,   42,   33,   48,   -6,
                      48,   20,   22,   25,   41,   22,   20,   57,
                       1,   16,   34,   22,    9,   28,   21,  -10,
                       5,   37,   17,   41,   10,   13,    0,  -20,
                      19,   27,   10,   16,   11,   10,   41,   34,
                      17,  -20,    4,   -8,   -4,   30,   -8,   46,
                ],
                [ // Black Bishop
                      62,   13,   22,   15,   31,   30,    8,   -2,
                      -3,   19,   11,   22,   32,    4,   -5,  -11,
                      49,   49,   48,   11,   26,   23,   16,   10,
                      32,   14,   18,   19,   18,   23,    7,   14,
                      12,   15,   12,   21,   17,   23,   17,   27,
                      15,   27,   25,   11,   17,   36,   36,   23,
                       2,   49,   12,   26,   11,   18,   40,   -4,
                      11,  -15,  -12,   39,   22,   -6,   25,   -6,
                ],
                [ // Black Queen
                      36,   80,   19,   55,   26,   31,   21,    6,
                      56,    9,   27,   59,   -5,    9,    9,  -15,
                      47,   45,   43,   21,   32,   15,    5,   -4,
                      21,   17,    9,   17,   10,    1,  -30,  -30,
                      17,   24,   18,   16,    5,   13,    0,  -13,
                      29,   14,  -10,   16,   -2,  -21,    0,  -14,
                      19,   -1,  -22,    0,  -11,    0,  -40,  -21,
                    -110,  -13,  -44,  -33,   -7,  -26,  -55,  -47,
                ],
                [ // Black King
                     -19,   19,   27,   10,  107,  112,  199,  317,
                      99,    2,    9,   58,   43,   49,   41,  280,
                      93,   43,   65,   72,   72,   73,   54,  143,
                      20,   35,   51,   67,   63,   51,   52,   97,
                      -4,   23,   34,   37,   42,   39,   28,   12,
                     -25,    2,   10,   15,   18,   28,   18,   21,
                     -25,   -6,    8,    2,    1,   16,    5,    7,
                     -51,   -7,  -35,    0,  -32,   -1,   -3,  -25,
                ],
                [ // Black Rook
                      30,   20,   24,   31,   34,   33,   39,   27,
                      30,   33,   37,   55,   43,   39,   36,   24,
                      10,   35,   41,   44,   37,   37,   36,   21,
                      17,    6,   37,   23,   25,   31,   34,   25,
                      -3,   14,   22,    5,   17,   20,   18,   30,
                      -1,    3,    7,   10,   17,   17,   16,    2,
                      -4,    8,   22,    5,   11,   20,    5,    6,
                     -24,   14,   16,   19,   15,   17,    8,   -9,
                ],
                [ // Black Uncastled Rook
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                       0,    0,    0,    0,    0,    0,    0,    0,
                ],
            ],
        }

        // Self {
            // chain_weight: 5,
            // passed_weight: 20,
            // doubled_weight: -15,
            // isolated_weight: -15,
            // king_pawn_weight: 10,

            // castle_bonus: 25,

            // knight_move_weight: 10,
            // bishop_move_weight: 10,
            // rook_move_weight: 5,
            // queen_move_weight: 2,
            // king_move_weight: 1,

            // pawn_weight: 100,
            // knight_weight: 320,
            // bishop_weight: 330,
            // rook_weight: 500,
            // queen_weight: 900,
            // king_weight: CHECKMATE,

            // psts: [[0; 64]; 16],
        // }
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
        board.b &= u64x4::splat(!(self.b ^ mov.b).reduce_or());
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
