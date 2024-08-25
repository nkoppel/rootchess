use crate::board::*;
use crate::eval::*;
use crate::gen_moves::*;
use crate::moves::*;
use crate::tt::*;

use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

#[path = "uci.rs"]
pub mod uci;

use crate::search::uci::*;

pub struct Searcher {
    gens: Vec<MoveGenerator>,
    c960: bool,
    nodes: usize,
    nodes_sec: usize,
    curr_depth: u8,
    time: u8,
    stop_time: Instant,
    prev_pos: HashMap<u64, u8>,
    history: [[[usize; 64]; 64]; 2],
    tt: TT,
    pawn_tt: TT,
    recv: Receiver<SearcherCommand>,
    stop: Receiver<bool>,
    id: usize,
}

fn pack_search(score: i32, time: u8, depth: u8, mov: Move) -> u64 {
    (score as u64) << 32 | (time as u64) << 24 | (depth as u64) << 16 | mov.0 as u64
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

pub fn ibv_exact(n: i32) -> i32 {
    (n + 1) & !3
}
pub fn ibv_min(n: i32) -> i32 {
    ((n + 1) & !3) + 1
}
pub fn ibv_max(n: i32) -> i32 {
    ((n + 1) & !3) - 1
}

pub fn ibv_is_exact(n: i32) -> bool {
    n & 3 == 0
}
pub fn ibv_is_lower(n: i32) -> bool {
    n & 3 == 1
}
pub fn ibv_is_upper(n: i32) -> bool {
    n & 3 == 3
}

pub fn from_ibv(n: i32) -> i32 {
    ibv_exact(n) / 4
}

pub fn show_ibv(n: i32) -> String {
    let mut out = String::new();

    match n & 3 {
        1 => out += "lowerbound ",
        3 => out += "upperbound ",
        _ => {}
    }

    out + &format!("cp {}", from_ibv(n))
}

impl Searcher {
    pub fn new(
        tt: TT,
        pawn_tt: TT,
        recv: Receiver<SearcherCommand>,
        stop: Receiver<bool>,
        id: usize,
    ) -> Self {
        Self {
            gens: Vec::new(),
            c960: false,
            nodes: 0,
            nodes_sec: 0,
            time: 0,
            stop_time: Instant::now() + Duration::from_secs(3155760000),
            curr_depth: 0,
            prev_pos: HashMap::new(),
            history: [[[0usize; 64]; 64]; 2],
            tt,
            pawn_tt,
            recv,
            stop,
            id,
        }
    }

    pub fn new_single(ttsize: usize, c960: bool) -> Self {
        Self {
            gens: Vec::new(),
            c960,
            nodes: 0,
            nodes_sec: 0,
            time: 0,
            stop_time: Instant::now() + Duration::from_secs(3155760000),
            curr_depth: 0,
            prev_pos: HashMap::new(),
            history: [[[0usize; 64]; 64]; 2],
            tt: TT::with_len(ttsize),
            pawn_tt: TT::with_len(1024),
            recv: channel().1,
            stop: channel().1,
            id: 0,
        }
    }

    pub fn incr_time(&mut self) {
        self.time = self.time.overflowing_add(1).0;
    }

    pub fn perft(&mut self, board: Board, depth: usize) -> u64 {
        if depth == 0 {
            1
        } else {
            let data = self.tt.read(board.hash);
            let (mut depth2, _) = (0, 0);

            if let Some(d) = data {
                let tmp = unpack_perft(d);
                depth2 = tmp.0;

                if depth2 as usize == depth {
                    return tmp.1;
                }
            }

            let mut generator = self.gens.pop().unwrap_or(MoveGenerator::empty());

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

        let mut generator = self.gens.pop().unwrap_or(MoveGenerator::empty());

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
        generator.moves.retain(|b| board.eval_see(b) >= 0);
        generator.moves.sort_by_cached_key(|b| -board.eval_see(b));
        let mut iter = generator.moves.drain(..);

        while let Some(board2) = iter.next() {
            score = -self.quiesce(board2, -beta, -alpha);

            if score >= cut {
                std::mem::drop(iter);
                self.gens.push(generator);
                return ibv_min(score);
            }
            if score > alpha {
                alpha = score;
            }
        }

        std::mem::drop(iter);
        self.gens.push(generator);
        alpha
    }

    fn write_tt(&mut self, hash: u64, score: i32, depth: u8, mov: Move) {
        let (hash2, res) = self.tt.force_read(hash);
        let (score2, time2, depth2, mov2) = unpack_search(res);

        if hash == hash2 {
            if depth >= depth2 {
                self.tt
                    .write(hash, pack_search(score, self.time, depth, mov));
            } else {
                self.tt
                    .write(hash, pack_search(score2, self.time, depth2, mov2));
            }
        } else if depth >= depth2 || self.time != time2 {
            self.tt
                .write(hash, pack_search(score, self.time, depth, mov));
        }
    }

    fn incr_prev_pos(&mut self, hash: u64) {
        if let Some(i) = self.prev_pos.get_mut(&hash) {
            *i += 1;
        } else {
            self.prev_pos.insert(hash, 1);
        }
    }

    fn decr_prev_pos(&mut self, hash: u64) {
        if let Some(i) = self.prev_pos.get_mut(&hash) {
            *i -= 1;

            if *i == 0 {
                self.prev_pos.remove(&hash);
            }
        }
    }

    pub fn alphabeta(
        &mut self,
        board: Board,
        mut alpha: i32,
        beta: i32,
        depth: u8,
    ) -> Result<i32, bool> {
        // Threefold repetition
        if let Some(1..) = self.prev_pos.get(&board.hash) {
            if depth != self.curr_depth {
                return Ok(0);
            }
        }

        // drop through into quiescense search
        if depth == 0 {
            return Ok(self.quiesce(board, alpha, beta));
        }

        let orig_alpha = alpha;

        let cut = ibv_exact(beta);
        let mut best_move = None;
        let mut pvs = false;

        if let Some(d) = self.tt.read(board.hash) {
            let (score, _, depth2, mov) = unpack_search(d);

            if depth2 >= depth {
                self.write_tt(board.hash, score, depth, mov);

                match score & 3 {
                    0 => return Ok(score),
                    1 if score >= cut => return Ok(score),
                    3 if score < alpha => return Ok(alpha),
                    _ => {}
                }
            }

            if mov != Move(0) {
                best_move = Some(board.do_move(mov));
            }
        }

        // Check for stop conditions
        if let Ok(b) = self.stop.try_recv() {
            return Err(b);
        } else if Instant::now() > self.stop_time {
            return Err(true);
        }

        // Null move Pruning
        if depth > 3 && !board.is_late_endgame() && !board.in_check() {
            let mut board2 = board.clone();
            board2.black ^= true;
            board2.remove_takeable_empty();
            board2.update_hash(&board);

            let score = -self.alphabeta(board2, -cut - 4, -cut, depth - 3)?;

            if score > cut {
                return Ok(score);
            }
        }

        let mut generator = self.gens.pop().unwrap_or(MoveGenerator::empty());

        generator.set_board(board.clone());
        generator.gen_moves();

        if generator.moves.is_empty() {
            if generator.get_checks() == 0 {
                return Ok(0);
            } else {
                return Ok(-CHECKMATE * 4);
            }
        }

        let mut moves = std::mem::take(&mut generator.moves);

        // Move Ordering
        if depth == self.curr_depth && self.id > 0 {
            let mut rng = thread_rng();

            moves.sort_by_cached_key(|b| {
                if Some(b.clone()) == best_move {
                    0
                } else {
                    rng.gen_range(1..1000000)
                }
            });
        } else {
            moves.sort_by_cached_key(|b| {
                if Some(b.clone()) == best_move {
                    -1000000
                } else if board.is_capture(b) {
                    -board.eval_see(b) as i64
                } else {
                    let mov = board.get_move(b, self.c960);
                    let history = self.history[board.black as usize][mov.start()][mov.end()];

                    1000000 - history as i64
                }
            });
        }

        let mut iter = moves.drain(..);
        let mut i = 0;

        while let Some(board2) = iter.next() {
            let mov = board.get_move(&board2, self.c960);

            // Extensions and Reductions
            let mut reduction = 1;

            // Late Move Reductions
            if !board.in_check() && !board2.in_check() && beta - alpha <= 4 {
                if depth > 2 && i > 3 {
                    reduction = 2;
                }
            } else if board2.in_check() {
                // Check extensions
                reduction = 0
            }

            // Principal Variation Search
            if pvs {
                self.incr_prev_pos(board.hash);
                let s = self.alphabeta(board2.clone(), -alpha - 4, -alpha, depth - reduction);
                self.decr_prev_pos(board.hash);

                s?;
                let s = -s.unwrap();

                if s <= alpha {
                    continue;
                }
            }

            // Alpha-Beta
            self.incr_prev_pos(board.hash);
            let score = self.alphabeta(board2.clone(), -beta, -alpha, depth - reduction);
            self.decr_prev_pos(board.hash);

            score?;
            let score = -score.unwrap();

            if score >= cut {
                std::mem::drop(iter);
                generator.moves = moves;
                self.gens.push(generator);

                let out = ibv_min(score);

                self.write_tt(board.hash, out, depth, mov);

                // History Heuristic
                if !board.is_capture(&board2) {
                    self.history[board.black as usize][mov.start()][mov.end()] +=
                        depth as usize * depth as usize;
                }

                return Ok(out);
            }
            if score > alpha {
                alpha = score;
                best_move = Some(board2.clone());
                pvs = true;
            }

            i += 1;
        }

        std::mem::drop(iter);
        generator.moves = moves;
        self.gens.push(generator);

        let mov = if let Some(b) = best_move {
            board.get_move(&b, self.c960)
        } else {
            Move(0)
        };

        if alpha != orig_alpha {
            self.write_tt(board.hash, alpha, depth, mov);
            Ok(alpha)
        } else {
            self.write_tt(board.hash, ibv_max(alpha), depth, mov);
            Ok(alpha)
        }
    }

    pub fn get_best_move(&self, board: &Board) -> Option<Move> {
        if let Some(d) = self.tt.read(board.hash) {
            let (.., mov) = unpack_search(d);

            if mov.0 != 0 {
                return Some(mov);
            }
        }
        None
    }

    pub fn show_pv(&self, depth: usize, board: &Board) {
        let mut board = board.clone();
        let mut i = 0;

        while let Some(d) = self.tt.read(board.hash) {
            let (.., mov) = unpack_search(d);

            if mov.0 != 0 {
                print!("{} ", mov);
                board = board.do_move(mov);
            } else {
                break;
            }

            i += 1;
            if i >= depth {
                break;
            }
        }
        println!();
    }

    pub fn search(&mut self, board: Board, min_depth: u8, max_depth: u8) -> i32 {
        let mut score = 0;
        let mut output_best_move = true;
        let mut best_move = None;
        let mut ponder_move = None;

        while self.stop.try_recv().is_ok() {}
        self.gens.clear();

        for depth in min_depth..=max_depth {
            self.curr_depth = depth;

            match self.alphabeta(board.clone(), -2000000, 2000000, depth) {
                Ok(s) => score = s,
                Err(o) => {
                    output_best_move = o;
                    break;
                }
            }

            if self.id == 0 {
                print!(
                    "info depth {} seldepth {} score {} pv ",
                    depth,
                    self.gens.len(),
                    show_ibv(score)
                );
                self.show_pv(depth as usize, &board);

                if let Some(mov1) = self.get_best_move(&board) {
                    best_move = Some(mov1);

                    if let Some(mov2) = self.get_best_move(&board.do_move(mov1)) {
                        ponder_move = Some(mov2);
                    }
                }
            }

            if score.abs() >= CHECKMATE * 4 - 1 && self.get_best_move(&board).is_some() {
                break;
            }
        }

        if self.id == 0 && output_best_move {
            if let Some(best) = best_move {
                print!("bestmove {} ", best);

                if let Some(ponder) = ponder_move {
                    print!("ponder {}", ponder);
                }
                println!();
            } else {
                println!("bestmove 0000");
            }
        }

        score
    }
}
