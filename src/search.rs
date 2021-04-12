use crate::gen_tables::*;
use crate::board::*;
use crate::gen_moves::*;
use crate::moves::*;
use crate::tt::*;
use crate::eval::*;

use std::sync::mpsc::{Receiver, channel};

pub struct Searcher {
    gens: Vec<MoveGenerator>,
    c960: bool,
    nodes: usize,
    nodes_sec: usize,
    time: u8,
    tt: TT,
    pawn_tt: TT
}

fn pack_search(score: i32, time: u8, depth: u8, mov: Move) -> u64 {
    (score as u64) << 32 |
    (time  as u64) << 24 |
    (depth as u64) << 16 |
     mov.0 as u64
}

fn unpack_search(te: u64) -> (i32, u8, u8, Move) {
    (
        (te >> 32) as i32,
        (te >> 24) as u8,
        (te >> 16) as u8,
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

pub fn ibv_exact(n: i32) -> i32 { (n + 1) & !3 }
pub fn ibv_min(n: i32)   -> i32 { (n + 1) & !3 + 1 }
pub fn ibv_max(n: i32)   -> i32 { (n + 1) & !3 - 1 }

impl Searcher {
    pub fn new(tt: TT, c960: bool) -> Self {
        Self {
            gens: Vec::new(),
            c960,
            nodes: 0,
            nodes_sec: 0,
            time: 0,
            tt,
            pawn_tt: TT::with_len(1024),
        }
    }

    pub fn perft(&mut self, board: Board, depth: usize) -> u64 {
        if depth == 0 {
            1
        } else {
            let data = self.tt.read(board.hash);
            let (mut depth2, mut cnt) = (0, 0);

            if let Some(d) = data {
                let tmp = unpack_perft(d);
                depth2 = tmp.0;
                cnt = tmp.1;

                if depth2 as usize == depth {
                    return cnt;
                }
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
                for mov in generator.moves.drain(..) {
                    out += self.perft(mov, depth - 1);
                }
            }

            if depth >= depth2 as usize && out < 1 << 56 {
                self.tt.write(board.hash, pack_perft(depth as u8, out));
            }

            self.gens.push(generator);
            out
        }
    }

    pub fn quiesce(&mut self, board: Board, mut alpha: i32, beta: i32) -> i32 {
        let cut = ibv_exact(beta);

        let mut generator =
            if let Some(g) = self.gens.pop() {
                g
            } else {
                MoveGenerator::empty()
            };

        let mut score = generator.eval(board.clone(), &mut self.pawn_tt) * 4;

        if score >= cut {
            self.gens.push(generator);
            return score + 1;
        }
        if score > alpha {
            alpha = score;
        }

        generator.set_board(board.clone());
        generator.gen_tactical();
        generator.moves.sort_by_cached_key(|b|
            invert_if(!board.black, board.eval_mvv_lva(b))
        );
        let mut iter = generator.moves.drain(..);

        loop {
            match iter.next() {
                Some(board2) => {
                    score = -self.quiesce(board2, -beta, -alpha);

                    if score >= cut {
                        std::mem::drop(iter);
                        self.gens.push(generator);
                        return score + 1;
                    }
                    if score > alpha {
                        alpha = score;
                    }
                },
                None => break,
            }
        }

        std::mem::drop(iter);
        self.gens.push(generator);
        alpha
    }

    fn write_tt(&mut self, hash: u64, score: i32, depth: u8, mov: Move) {
        let (hash2, res) = self.tt.force_read(hash);
        let (score2, time2, depth2, mov2) = unpack_search(res);

        if hash2 == hash && depth2 > depth && time2 != self.time {
            self.tt.write(hash, pack_search(score2, self.time, depth2, mov2));
        } else if depth >= depth2 || time2 != self.time {
            self.tt.write(hash, pack_search(score, self.time, depth, mov));
        }
    }

    pub fn alphabeta(&mut self,
                     board: Board,
                     mut alpha: i32,
                     mut beta: i32,
                     depth: u8,
                     stop: &Receiver<()>)
        -> Option<i32>
    {
        if depth == 0 {
            return Some(self.quiesce(board, alpha, beta))
        }

        let cut = ibv_exact(beta);
        let mut best_move = None;
        let mut score2 = ibv_min(alpha);

        if let Some(d) = self.tt.read(board.hash) {
            let (score, _, depth2, mov) = unpack_search(d);

            score2 = score;

            if depth2 >= depth {
                match score & 3 {
                    0 | 1 if score >= beta  => return Some(score),
                    1     if score >  alpha => alpha = score,
                    3     if score <  beta  => beta  = score,
                    _ => {}
                }
            }

            if mov != Move(0) {
                best_move = Some(board.do_move(mov));
            }
        }

        if let Ok(()) = stop.try_recv() {
            return None;
        }

        let mut generator =
            if let Some(g) = self.gens.pop() {
                g
            } else {
                MoveGenerator::empty()
            };

        generator.set_board(board.clone());
        generator.gen_moves();

        let mut moves = std::mem::take(&mut generator.moves);
        moves.sort_by_cached_key(|b| {
            if Some(b.clone()) == best_move {
                -100000
            } else {
                invert_if(!board.black, board.eval_mvv_lva(b))
            }
        });

        generator.moves = moves;

        if generator.moves.is_empty() {
            if generator.get_checks() == 0 {
                return Some(0);
            } else {
                return Some(-25600 * 4);
            }
        }

        let mut iter = generator.moves.drain(..);

        loop {
            match iter.next() {
                Some(board2) => {
                    if let Some(_) = best_move {
                        if let Some(s) = self.alphabeta(board2.clone(), -alpha - 1, -alpha, depth - 1, stop) {
                            if -s <= alpha {
                                continue;
                            }
                        } else {
                            return None;
                        }
                    }

                    match self.alphabeta(board2.clone(), -beta, -alpha, depth - 1, stop) {
                        Some(mut score) => {
                            score = -score;

                            if score >= cut {
                                std::mem::drop(iter);
                                self.gens.push(generator);

                                let out = ibv_min(score);
                                let mov =
                                    if let Some(b) = best_move {
                                        board.get_move(&b, self.c960)
                                    } else {
                                        Move(0)
                                    };

                                self.write_tt(board.hash, out, depth, mov);
                                return Some(out);
                            }
                            if score > alpha {
                                alpha = score;
                                best_move = Some(board2.clone());
                            }
                        }
                        None => return None
                    }
                },
                None => break,
            }
        }

        std::mem::drop(iter);
        self.gens.push(generator);

        let mov =
            if let Some(b) = best_move {
                board.get_move(&b, self.c960)
            } else {
                Move(0)
            };

        self.write_tt(board.hash, alpha, depth, mov);
        Some(alpha)
    }

    pub fn get_best_move(&self, board: &Board) -> Option<Move> {
        if let Some(d) = self.tt.read(board.hash) {
            let (.., mov) = unpack_search(d);

            if mov.0 != 0 {
                return Some(mov)
            }
        }
        None
    }

    pub fn search(&mut self, board: Board, min_depth: u8, max_depth: u8)
        -> i32
    {
        let mut score = Some(0);
        // let mut alpha;
        // let mut beta;
        let recv = channel().1;

        for depth in min_depth..=max_depth {
            println!("{}", depth);
            // alpha = score - 50;
            // beta  = score + 50;

            score =
                self.alphabeta(board.clone(), -1000000, 1000000, depth, &recv);

            // loop {
                // let score2 =
                    // self.alphabeta(board.clone(), alpha, beta, depth, &recv).unwrap();

                // // println!("{} {} {} {}", score, score2, alpha, beta);

                // if score2 <= alpha {
                    // alpha += 3 * (alpha - score);
                // } else if score2 >= beta {
                    // beta += 3 * (beta - score);
                // } else {
                    // score = score2;
                    // break;
                // }
            // }
        }

        score.unwrap()
    }
}
