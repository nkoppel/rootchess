use crate::gen_tables::*;
use crate::board::*;
use crate::gen_moves::*;
use crate::moves::*;

//hash, score/bound, pv move, depth
//64, 32, 16, 16

pub struct Searcher {
    gens: Vec<MoveGenerator>,
    c960: bool,
    tt: Vec<(u64, u64)>,
    tmask: u64
}

fn pack_search(score: i32, depth: u16, mov: Move) -> u64 {
    (score as u64) << 32 |
    (depth as u64) << 16 |
     mov.0 as u64
}

fn unpack_search(te: u64) -> (i32, u16, Move) {
    (
        (te >> 32) as i32,
        (te >> 16) as u16,
        Move(te as u16),
    )
}

fn pack_perft(depth: u8, mut cnt: u64) -> u64 {
    cnt &= 0x00ffffffffffffff;
    cnt |= (depth as u64) << 56;
    cnt
}

fn unpack_perft(te: u64) -> (u8, u64) {
    ((te >> 56) as u8, te & 0x00ffffffffffffff)
}

impl Searcher {
    pub fn new(tablebits: usize) -> Self {
        Self {
            gens: Vec::new(),
            tt: vec![(0,0); 1 << tablebits],
            tmask: (1 << tablebits) - 1,
        }
    }

    pub fn perft(&mut self, board: Board, depth: usize) -> u64 {
        if depth == 0 {
            1
        } else {
            let ind = (board.hash & self.tmask) as usize;
            let (hash, data) = self.tt[ind];
            let (depth2, cnt) = unpack_perft(data);

            if hash == board.hash && depth2 as usize == depth {
                return cnt;
            }

            let mut generator =
                if let Some(g) = self.gens.pop() {
                    g
                } else {
                    MoveGenerator::empty()
                };

            generator.set_board(board.clone());
            generator.gen_moves();

            let mut out = 0;

            if depth == 1 {
                out = generator.moves.len() as u64;
            } else {
                for mov in generator.moves.iter().cloned() {
                    out += self.perft(mov, depth - 1);
                }
            }

            if depth >= depth2 as usize && out < 1 << 56 {
                self.tt[ind] = (board.hash, pack_perft(depth as u8, out));
            }

            self.gens.push(generator);
            out
        }
    }
}
