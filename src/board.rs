use packed_simd::*;

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Board(u64x4);

impl Board {
    pub fn get_piece(&self, piece: u8) -> u64 {
        let mut vec = u64x4::splat(piece as u64);

        vec >>= u64x4::new(3, 2, 1, 0);
        vec &= 1;
        vec *= u64::MAX;

        vec = vec ^ self.0;
        !vec.or()
    }

    pub fn get_square(&self, sq: u8) -> u8 {
        let mut vec = self.0;

        vec <<= 64 - sq as u32;
        vec.bitmask()
    }

    pub fn to_squarewise(&self) -> Vec<u8> {
        let mut out = vec![0; 64];

        for i in 0..64 {
            out[i] = self.get_square(i as u8);
        }

        out
    }

    pub fn from_squarewise(squares: &[u8]) -> Self {
        let mut out = vec![0; 4];

        for i in 0..4 {
            for j in 0..64 {
                out[3 - i] |= ((squares[j] >> i & 1) as u64) << j;
            }
        }

        Board(u64x4::from_slice_aligned(&out[..]))
    }

    pub fn get_rooks(&self) -> u64 {
        let mut vec = self.0;
        vec |= u64x4::new(0, 0, 0, u64::MAX);
        vec.and()
    }

    pub fn get_occ(&self) -> u64 {
        let mut vec = self.0;
        vec &= u64x4::new(0, u64::MAX, u64::MAX, u64::MAX);
        vec.or()
    }

    pub fn get_black(&self) -> u64 {
        unsafe {
            self.get_occ() & self.0.extract_unchecked(0)
        }
    }

    pub fn get_white(&self) -> u64 {
        unsafe {
            self.get_occ() & !self.0.extract_unchecked(0)
        }
    }

    pub fn remove_takeable_empty(&mut self) {
        self.0 &= !self.get_piece(0x8);
    }
}
