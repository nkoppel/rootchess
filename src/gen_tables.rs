const BISHOP_MAGICS: [u64; 64] =
[
    0x007bfeffbfeffbff, 0x003effbfeffbfe08, 0x0000401020200000, 0x0000200810000000, 0x0000110080000000, 0x0000080100800000, 0x0007efe0bfff8000, 0x00000fb0203fff80,
    0x00007dff7fdff7fd, 0x0000011fdff7efff, 0x0000004010202000, 0x0000002008100000, 0x0000001100800000, 0x0000000801008000, 0x000007efe0bfff80, 0x000000080f9fffc0,
    0x0000400080808080, 0x0000200040404040, 0x0000400080808080, 0x0000200200801000, 0x0000240080840000, 0x0000080080840080, 0x0000040010410040, 0x0000020008208020,
    0x0000804000810100, 0x0000402000408080, 0x0000804000810100, 0x0000404004010200, 0x0000404004010040, 0x0000101000804400, 0x0000080800104100, 0x0000040400082080,
    0x0000410040008200, 0x0000208020004100, 0x0000110080040008, 0x0000020080080080, 0x0000404040040100, 0x0000202040008040, 0x0000101010002080, 0x0000080808001040,
    0x0000208200400080, 0x0000104100200040, 0x0000208200400080, 0x0000008840200040, 0x0000020040100100, 0x007fff80c0280050, 0x0000202020200040, 0x0000101010100020,
    0x0007ffdfc17f8000, 0x0003ffefe0bfc000, 0x0000000820806000, 0x00000003ff004000, 0x0000000100202000, 0x0000004040802000, 0x007ffeffbfeff820, 0x003fff7fdff7fc10,
    0x0003ffdfdfc27f80, 0x000003ffefe0bfc0, 0x0000000008208060, 0x0000000003ff0040, 0x0000000001002020, 0x0000000040408020, 0x00007ffeffbfeff9, 0x007ffdff7fdff7fd,
];

const BISHOP_OFFSETS: [u64; 64] = [
    16530, 9162, 9674, 18532, 19172, 17700, 5730, 19661,
    17065, 12921, 15683, 17764, 19684, 18724, 4108, 12936,
    15747, 4066, 14359, 36039, 20457, 43291, 5606, 9497,
    15715, 13388, 5986, 11814, 92656, 9529, 18118, 5826,
     4620, 12958, 55229, 9892, 33767, 20023, 6515, 6483,
    19622, 6274, 18404, 14226, 17990, 18920, 13862, 19590,
     5884, 12946, 5570, 18740, 6242, 12326, 4156, 12876,
    17047, 17780, 2494, 17716, 17067, 9465, 16196, 6166
];

const ROOK_MAGICS: [u64; 64] =
[
    0x00a801f7fbfeffff, 0x00180012000bffff, 0x0040080010004004, 0x0040040008004002, 0x0040020004004001, 0x0020008020010202, 0x0040004000800100, 0x0810020990202010,
    0x000028020a13fffe, 0x003fec008104ffff, 0x00001800043fffe8, 0x00001800217fffe8, 0x0000200100020020, 0x0000200080010020, 0x0000300043ffff40, 0x000038010843fffd,
    0x00d00018010bfff8, 0x0009000c000efffc, 0x0004000801020008, 0x0002002004002002, 0x0001002002002001, 0x0001001000801040, 0x0000004040008001, 0x0000802000200040,
    0x0040200010080010, 0x0000080010040010, 0x0004010008020008, 0x0000020020040020, 0x0000010020020020, 0x0000008020010020, 0x0000008020200040, 0x0000200020004081,
    0x0040001000200020, 0x0000080400100010, 0x0004010200080008, 0x0000200200200400, 0x0000200100200200, 0x0000200080200100, 0x0000008000404001, 0x0000802000200040,
    0x00ffffb50c001800, 0x007fff98ff7fec00, 0x003ffff919400800, 0x001ffff01fc03000, 0x0000010002002020, 0x0000008001002020, 0x0003fff673ffa802, 0x0001fffe6fff9001,
    0x00ffffd800140028, 0x007fffe87ff7ffec, 0x003fffd800408028, 0x001ffff111018010, 0x000ffff810280028, 0x0007fffeb7ff7fd8, 0x0003fffc0c480048, 0x0001ffffa2280028,
    0x00ffffe4ffdfa3ba, 0x007ffb7fbfdfeff6, 0x003fffbfdfeff7fa, 0x001fffeff7fbfc22, 0x000ffffbf7fc2ffe, 0x0007fffdfa03ffff, 0x0003ffdeff7fbdec, 0x0001ffff99ffab2f,
];

const ROOK_OFFSETS: [u64; 64] = [
    85487, 43101, 0, 49085, 93168, 78956, 60703, 64799,
    30640, 9256, 28647, 10404, 63775, 14500, 52819, 2048,
    52037, 16435, 29104, 83439, 86842, 27623, 26599, 89583,
    7042, 84463, 82415, 95216, 35015, 10790, 53279, 70684,
    38640, 32743, 68894, 62751, 41670, 25575, 3042, 36591,
    69918, 9092, 17401, 40688, 96240, 91632, 32495, 51133,
    78319, 12595, 5152, 32110, 13894, 2546, 41052, 77676,
    73580, 44947, 73565, 17682, 56607, 56135, 44989, 21479
];

pub struct BitStack(pub u64);

impl Iterator for BitStack {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let out = self.0 & !(self.0 - 1);
            self.0 ^= out;
            Some(out)
        }
    }
}

pub struct LocStack(pub u64);

impl Iterator for LocStack {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            None
        } else {
            let out = self.0.trailing_zeros();
            self.0 ^= 1 << out;
            Some(out as usize)
        }
    }
}

fn num_to_mask(num: u64, mask: u64) -> u64 {
    let mut num_bit = 1;
    let mut out = 0;

    for mask_bit in BitStack(mask) {
        if num & num_bit != 0 {
            out |= mask_bit;
        }
        num_bit <<= 1;
    }

    out
}

fn ray_att(sq: usize, delta: (isize, isize), board: u64, out: &mut u64) {
    let mut x = (sq % 8) as isize;
    let mut y = (sq / 8) as isize;
    let (dx, dy) = delta;
    x += dx;
    y += dy;
    while x >= 0 && x < 8 && y >= 0 && y < 8 && board & (1 << (x + y * 8)) == 0
    {
        *out |= 1 << (x + y * 8);
        x += dx;
        y += dy;
    }

    if x >= 0 && x < 8 && y >= 0 && y < 8 {
        *out |= 1 << (x + y * 8);
    }
}

fn gen_att(sq: usize, deltas: &[(isize, isize)], board: u64) -> u64 {
    let mut out = 0;

    for delta in deltas {
        ray_att(sq, *delta, board, &mut out);
    }

    out
}

pub fn ray_mask(sq: usize, delta: (isize, isize), board: &mut u64) {
    let mut x = (sq % 8) as isize;
    let mut y = (sq / 8) as isize;
    let (dx, dy) = delta;

    let mut prev_borders = (x == 0) as usize + (x == 7) as usize +
                           (y == 0) as usize + (y == 7) as usize;
    let p_x_border = x == 0 || x == 7;
    let p_y_border = y == 0 || y == 7;
    x += dx;
    y += dy;

    if x < 0 || x >= 8 || y < 0 || y >= 8 {
        return;
    }

    let mut borders = (x == 0) as usize + (x == 7) as usize +
                      (y == 0) as usize + (y == 7) as usize;
    let c_x_border = x == 0 || x == 7;
    let c_y_border = y == 0 || y == 7;

    if prev_borders == borders &&
        (p_x_border != c_x_border || p_y_border != c_y_border)
    {
        return;
    }

    while borders <= prev_borders {
        *board |= 1 << (x + y * 8);
        x += dx;
        y += dy;
        prev_borders = borders;
        borders = (x == 0) as usize + (x == 7) as usize +
                  (y == 0) as usize + (y == 7) as usize;
    }
}

fn gen_mask(sq: usize, deltas: &[(isize, isize)]) -> u64 {
    let mut board = 0;

    for delta in deltas {
        ray_mask(sq, *delta, &mut board);
    }

    board
}

fn deltas(bishop: bool) -> Vec<(isize, isize)> {
    if bishop {
        vec![(1, 1), (1, -1), (-1, 1), (-1, -1)]
    } else {
        vec![(1, 0), (-1, 0), (0, 1), (0, -1)]
    }
}

fn gen_masks(bishop: bool) -> Vec<u64> {
    let deltas = deltas(bishop);

    let mut out = vec![0; 64];

    for sq in 0..64 {
        out[sq] = gen_mask(sq, &deltas);
    }

    out
}

fn gen_occ_att(sq: usize, bishop: bool) -> Vec<(u64, u64)> {
    let deltas = deltas(bishop);
    let mask = gen_mask(sq, &deltas);
    let size = 1 << mask.count_ones();
    let mut out = Vec::new();

    for i in 0..size {
        let o = num_to_mask(i as u64, mask);
        out.push((o, gen_att(sq, &deltas, o)));
    }
    out
}

pub fn print_board(board: u64) {
    println!("{:#x}", board);
    for y in (0..8).rev() {
        for x in 0..8 {
            if board & (1 << (x + y * 8)) != 0 {
                print!("X ");
            } else {
                print!(". ");
            }
        }
        println!();
    }
}

fn gen_sliding_table(bishop: bool) -> Vec<(u64, u64, u64)> {
    let masks = gen_masks(bishop);
    let mut out = Vec::new();

    for i in 0..64 {
        if bishop {
            out.push((masks[i], BISHOP_MAGICS[i], BISHOP_OFFSETS[i]));
        } else {
            out.push((masks[i], ROOK_MAGICS[i], ROOK_OFFSETS[i]));
        }
    }

    out
}

fn gen_magic_table() -> Vec<u64> {
    let mut out = vec![0; 97264];
    for sq in 0..64 {
        for (mut occ, att) in gen_occ_att(sq, true) {
            occ = occ.overflowing_mul(BISHOP_MAGICS[sq]).0;
            occ >>= 55;
            occ += BISHOP_OFFSETS[sq];
            if out[occ as usize] == 0 {
                out[occ as usize] = att;
            } else if out[occ as usize] != att {
                panic!("Invalid magics!");
            }
        }

        for (mut occ, att) in gen_occ_att(sq, false) {
            occ = occ.overflowing_mul(ROOK_MAGICS[sq]).0;
            occ >>= 52;
            occ += ROOK_OFFSETS[sq];
            if out[occ as usize] == 0 {
                out[occ as usize] = att;
            } else if out[occ as usize] != att {
                panic!("Invalid magics!");
            }
        }
    }

    out
}

fn gen_move_table(deltas: &[(isize, isize)]) -> Vec<u64> {
    let mut out = vec![0; 64];

    for sq in 0..64 {
        let mut board = 0;
        let x = sq % 8;
        let y = sq / 8;

        for (dx, dy) in deltas {
            let tx = x + dx;
            let ty = y + dy;

            if tx >= 0 && tx < 8 && ty >= 0 && ty < 8 {
                board |= 1 << (tx + ty * 8);
            }
        }
        out[sq as usize] = board;
    }

    out
}

pub struct Tables {
    pub white_pawn_takes: Vec<u64>,
    pub white_pawn_moves: Vec<u64>,
    pub black_pawn_takes: Vec<u64>,
    pub black_pawn_moves: Vec<u64>,
    pub bishop: Vec<(u64, u64, u64)>,
    pub rook: Vec<(u64, u64, u64)>,
    pub knight: Vec<u64>,
    pub king: Vec<u64>,
    pub magic: Vec<u64>,
}

pub fn new_tables() -> Tables {
    Tables {
        white_pawn_takes: gen_move_table(&vec![(-1, 1), (1, 1)]),
        white_pawn_moves: gen_move_table(&vec![(0, 1)]),
        black_pawn_takes: gen_move_table(&vec![(-1, -1), (1, -1)]),
        black_pawn_moves: gen_move_table(&vec![(0, -1)]),
        knight: gen_move_table(&vec![(1, 2), (-1, 2), (1, -2), (-1, -2), (2, 1), (-2, 1), (2, -1), (-2, -1)]),
        king: gen_move_table(&vec![(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]),
        bishop: gen_sliding_table(true),
        rook: gen_sliding_table(false),
        magic: gen_magic_table(),
    }
}
