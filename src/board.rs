use crate::gen_tables::*;
use packed_simd::*;
use packed_simd::*;

macro_rules! gen_line {
    ( $val:ident, $op:tt, 0, 0, 0, 0 ) => {};
    ( $val:ident, $op:tt, $($vec:expr),+ ) => {
        $val $op u64x4::new($($vec),+);
    };
}

macro_rules! get_piece {
    ($name:ident, [ $($or_vec:expr),+ ], [ $($xor_vec:expr),+ ]) => {
        #[allow(unused)]
        pub fn $name(&self) -> u64 {
            let mut vec = self.b;
            gen_line!(vec, |=, $($or_vec),+);
            gen_line!(vec, ^=, $($xor_vec),+);
            vec.and()
        }
    }
}

/* square definitions
 *  0 = empty
 *  1 = white pawn
 *  2 = white knight
 *  3 = white bishop
 *  4 = white queen
 *  5 = white king
 *  6 = white rook
 *  7 = uncastled white rook
 *  8 = takeable empty (en-passant)
 *  9 = black pawn
 *  A = black knight
 *  B = black bishop
 *  C = black queen
 *  D = black king
 *  E = black rook
 *  F = uncastled black rook
 */

const M: u64 = u64::MAX;
const FEN_PIECES: &str = "_PNBQKRR_pnbqkrr";
pub const FILES: &str = "hgfedcba";

pub fn sq_from_str(s: &str) -> usize {
    let mut word = s.chars();

    let c1 = word.next().unwrap();
    let c2 = word.next().unwrap();

    FILES.find(c1).unwrap() + c2.to_digit(9).unwrap() as usize * 8 - 8
}

pub fn str_from_sq(sq: usize) -> String {
    let mut out = String::new();
    let file = sq % 8;

    out += &FILES[file..file + 1];
    out += &format!("{}", sq / 8);

    out
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Board{
    pub b: u64x4,
    pub black: bool,
    pub hash: u64
}

impl Board {
    pub fn get_piece(&self, piece: u8) -> u64 {
        let mut vec = u64x4::splat(piece as u64);

        vec >>= u64x4::new(3, 2, 1, 0);
        vec &= 1;
        vec *= u64::MAX;

        vec ^= self.b;
        !vec.or()
    }

    pub fn get_square(&self, sq: u8) -> u8 {
        let mut vec = self.b;

        vec >>= sq as u32;
        vec &= 1;
        vec <<= u64x4::new(3, 2, 1, 0);

        vec.or() as u8
    }

    pub fn to_squarewise(&self) -> Vec<u8> {
        let mut out = vec![0; 64];

        for i in 0..64 {
            out[i] = self.get_square(i as u8);
        }

        out
    }

    pub fn from_squarewise(squares: &[u8], black: bool, hasher: &Hasher)
        -> Self
    {
        let mut out = vec![0; 4];

        for i in 0..4 {
            for j in 0..64 {
                out[3 - i] |= ((squares[j] >> i & 1) as u64) << j;
            }
        }

        let mut out = Board {
            b: u64x4::from_slice_unaligned(&out[..]),
            black,
            hash: 0
        };

        out.init_hash(hasher);
        out
    }

    pub fn to_fen(&self, c960: bool) -> String {
        let squares = self.to_squarewise();
        let mut out = String::new();

        for y in (0..8).rev() {
            let mut empty = 0;

            for x in (0..8).rev() {
                let sq = squares[x + y * 8] as usize;

                if sq % 8 == 0 {
                    empty += 1;
                } else {
                    if empty > 0 {
                        out += &format!("{}", empty);
                        empty = 0;
                    }
                    out += &FEN_PIECES[sq..sq + 1];
                }
            }

            if empty > 0 {
                out += &format!("{}", empty);
            }

            if y > 0 {
                out += "/";
            }
        }

        if self.black {
            out += " b ";
        } else {
            out += " w ";
        }

        for sq in LocStack(self.castling_white_rooks()) {
            if c960 {
                let file = sq % 8;
                out += &FILES[file..file + 1].to_ascii_uppercase();
            } else if sq == 0 {
                out += "K"
            } else if sq == 7 {
                out += "Q"
            }
        }

        for sq in LocStack(self.castling_black_rooks()) {
            if c960 {
                let file = sq % 8;
                out += &FILES[file..file + 1];
            } else if sq == 56 {
                out += "k"
            } else if sq == 63 {
                out += "q"
            }
        }

        if &out[out.len() - 1 ..] == " " {
            out += "-";
        }

        out += " ";

        let ep_squares = self.takeable_empties();

        if ep_squares != 0 {
            out += &str_from_sq(ep_squares.trailing_zeros() as usize);
            out += " ";
        } else {
            out += "- ";
        }

        out
    }

    pub fn from_fen(fen: &str, hasher: &Hasher) -> Self {
        let mut squares = vec![0u8; 64];
        let mut words = fen.split(' ');

        let mut y = 7;
        let mut x = 7;

        for c in words.next().unwrap().chars() {
            match c {
                '/' => {y -= 1; x = 7}
                _ if c.is_digit(8) => {
                    let d = c.to_digit(8).unwrap();

                    if x >= d {
                        x -= d
                    }
                }
                _ => {
                    if let Some(i) = FEN_PIECES.find(c) {
                        squares[(x + y * 8) as usize] = i as u8;

                        if x > 0 {
                            x -= 1;
                        }
                    }
                }
            }
        }

        let mut black = false;

        if words.next().unwrap().chars().next().unwrap() == 'b' {
            black = true;
        }

        for c in words.next().unwrap().chars() {
            let (sq_offset, piece) =
                if c.is_ascii_lowercase() {
                    (56, 0xF)
                } else {
                    (0, 7)
                };

            match c.to_ascii_lowercase() {
                'k' => {squares[sq_offset]     = piece}
                'q' => {squares[sq_offset + 7] = piece}
                f => {
                    if let Some(f) = FILES.find(f) {
                        squares[sq_offset + f] = piece
                    }
                }
            }
        }

        squares[sq_from_str(&words.next().unwrap())] = 8;

        Board::from_squarewise(&squares, black, hasher)
    }

    get_piece!(pawns  , [M, 0, 0, 0], [0, M, M, 0]);
    get_piece!(knights, [M, 0, 0, 0], [0, M, 0, M]);
    get_piece!(bishops, [M, 0, 0, 0], [0, M, 0, 0]);
    get_piece!(queens , [M, 0, 0, 0], [0, 0, M, M]);
    get_piece!(kings  , [M, 0, 0, 0], [0, 0, M, 0]);

    get_piece!(white_pawns  , [0, 0, 0, 0], [M, M, M, 0]);
    get_piece!(white_knights, [0, 0, 0, 0], [M, M, 0, M]);
    get_piece!(white_bishops, [0, 0, 0, 0], [M, M, 0, 0]);
    get_piece!(white_queens , [0, 0, 0, 0], [M, 0, M, M]);
    get_piece!(white_kings  , [0, 0, 0, 0], [M, 0, M, 0]);

    get_piece!(black_pawns  , [0, 0, 0, 0], [0, M, M, 0]);
    get_piece!(black_knights, [0, 0, 0, 0], [0, M, 0, M]);
    get_piece!(black_bishops, [0, 0, 0, 0], [0, M, 0, 0]);
    get_piece!(black_queens , [0, 0, 0, 0], [0, 0, M, M]);
    get_piece!(black_kings  , [0, 0, 0, 0], [0, 0, M, 0]);

    get_piece!(rooks      , [M, 0, 0, M], [0, 0, 0, 0]);
    get_piece!(white_rooks, [0, 0, 0, M], [M, 0, 0, 0]);
    get_piece!(black_rooks, [0, 0, 0, M], [0, 0, 0, 0]);

    get_piece!(castling_rooks      , [M, 0, 0, 0], [0, 0, 0, 0]);
    get_piece!(castling_white_rooks, [0, 0, 0, 0], [M, 0, 0, 0]);
    get_piece!(castling_black_rooks, [0, 0, 0, 0], [0, 0, 0, 0]);

    get_piece!(takeable_empties, [0, 0, 0, 0], [0, M, M, M]);

    pub fn occ(&self) -> u64 {
        let mut vec = self.b;
        vec &= u64x4::new(0, M, M, M);
        vec.or()
    }

    pub fn black(&self) -> u64 {
        unsafe {
            self.occ() & self.b.extract_unchecked(0)
        }
    }

    pub fn white(&self) -> u64 {
        unsafe {
            self.occ() & !self.b.extract_unchecked(0)
        }
    }

    pub fn remove_takeable_empty(&mut self) {
        self.b &= !self.takeable_empties();
    }
}

pub struct Hasher {
    bits: [[u64; 64]; 4],
    black: u64,
}

use rand::{Rng, thread_rng};

impl Hasher {
    pub fn new() -> Self {
        let mut rng = thread_rng();
        let mut bits = [[0u64; 64]; 4];

        for i in 0..4 {
            for j in 0..64 {
                bits[i][j] = rng.gen();
            }
        }

        Self {
            bits,
            black: rng.gen()
        }
    }

    fn hash_bits(&self, bits: u64x4) -> u64 {
        let mut hash = 0;

        for i in 0..4 {
            for j in LocStack(unsafe{bits.extract_unchecked(i)}) {
                hash ^= self.bits[i][j];
            }
        }

        hash
    }
}

impl Board {
    pub fn init_hash(&mut self, hasher: &Hasher) {
        let mut hash = hasher.hash_bits(self.b);

        if self.black {
            hash ^= hasher.black;
        }

        self.hash = hash;
    }

    pub fn update_hash(&mut self, prev: &Board, hasher: &Hasher) {
        let mut hash = hasher.hash_bits(self.b ^ prev.b);

        if self.black != prev.black {
            hash ^= hasher.black;
        }

        self.hash ^= hash;
    }
}
