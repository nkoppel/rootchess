mod positions_from_games;

pub use positions_from_games::positions_from_games;

use crate::gen_moves::*;
use crate::eval::*;
use crate::board::*;
use crate::tt::*;
use crate::search::*;

use std::fs::File;
use std::io::{self, BufRead, BufReader};

fn read_positions(file: &str) -> Vec<(f64, Board)> {
    let mut out = Vec::new();

    for line in BufReader::new(File::open(file).unwrap()).lines() {
        let line = line.unwrap();
        let ind = line.find(' ').unwrap();

        out.push((line[..ind].parse::<f64>().unwrap(), Board::from_fen(&line[ind + 1..])));
    }

    out
}

use std::convert::TryInto;

impl EvalParams {
    fn from_vec(vec: &[i32]) -> Self {
        EvalParams {
            chain_weight: vec[0],
            passed_weight: vec[1],
            doubled_weight: vec[2],
            isolated_weight: vec[3],
            king_pawn_weight: vec[4],

            castle_bonus: vec[5],

            knight_move_weight: vec[6],
            bishop_move_weight: vec[7],
            rook_move_weight  : vec[8],
            queen_move_weight : vec[9],
            king_move_weight  : vec[10],

            pawn_weight: 100,
            knight_weight: 279,
            bishop_weight: 293,
            rook_weight: 466,
            queen_weight: 866,
            king_weight: 25600,

            psts: {
                let mut psts = [[0; 64]; 16];

                for (i, j) in [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14].iter().enumerate() {
                    let start = i * 64 + 11;

                    psts[*j] = vec[start..start + 64].try_into().unwrap();
                }

                psts
            }
        }
    }

    fn to_vec(&self) -> Vec<i32> {
        let mut out = Vec::new();

        out.push(self.chain_weight);
        out.push(self.passed_weight);
        out.push(self.doubled_weight);
        out.push(self.isolated_weight);
        out.push(self.king_pawn_weight);

        out.push(self.castle_bonus);

        out.push(self.knight_move_weight);
        out.push(self.bishop_move_weight);
        out.push(self.rook_move_weight);
        out.push(self.queen_move_weight);
        out.push(self.king_move_weight);

        for i in [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14] {
            out.extend(self.psts[i]);
        }

        out
    }

    // fn from_vec(vec: &[i32]) -> Self {
        // EvalParams {
            // chain_weight: vec[0],
            // passed_weight: vec[1],
            // doubled_weight: vec[2],
            // isolated_weight: vec[3],
            // king_pawn_weight: vec[4],

            // castle_bonus: vec[5],

            // knight_move_weight: vec[6],
            // bishop_move_weight: vec[7],
            // rook_move_weight  : vec[8],
            // queen_move_weight : vec[9],
            // king_move_weight  : vec[10],

            // pawn_weight: vec[11],
            // knight_weight: vec[12],
            // bishop_weight: vec[13],
            // rook_weight: vec[14],
            // queen_weight: vec[15],
            // king_weight: 25600,

            // psts: [[0; 64]; 16]
        // }
    // }

    // fn to_vec(&self) -> Vec<i32> {
        // let mut out = Vec::new();

        // out.push(self.chain_weight);
        // out.push(self.passed_weight);
        // out.push(self.doubled_weight);
        // out.push(self.isolated_weight);
        // out.push(self.king_pawn_weight);

        // out.push(self.castle_bonus);

        // out.push(self.knight_move_weight);
        // out.push(self.bishop_move_weight);
        // out.push(self.rook_move_weight);
        // out.push(self.queen_move_weight);
        // out.push(self.king_move_weight);

        // out.push(self.pawn_weight);
        // out.push(self.knight_weight);
        // out.push(self.bishop_weight);
        // out.push(self.rook_weight);
        // out.push(self.queen_weight);

        // out
    // }
}

#[derive(Clone, Debug)]
struct ParamInfo {
    index: usize,
    increment: i32,
    can_backoff: bool,
    prev_backoff: usize,
    backoff: usize,
}

use std::cmp::Ordering;

impl PartialEq for ParamInfo {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.backoff == other.backoff
    }
}

impl Eq for ParamInfo { }

impl Ord for ParamInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        let ord1 = self.backoff.cmp(&other.backoff);

        if ord1 == Ordering::Equal {
            self.index.cmp(&other.index)
        } else {
            ord1
        }
    }
}

impl PartialOrd for ParamInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn tune_abstract_backoff<F>(params: &mut Vec<i32>, mut error: F)
    where F: FnMut(&[i32]) -> f64
{
    println!("Begin tuning");

    let mut infos = Vec::new();

    let mut best_error = error(&params);
    let mut time_since_improved = 0;

    for index in 0..params.len() {
        infos.push(ParamInfo {
            index,
            increment: 1,
            can_backoff: false,
            prev_backoff: 0,
            backoff: 0,
        });
    }

    for i in 0.. {
        println!("vec!{:?}", params);
        println!("Begin iteration {}, error = {}", i, best_error);

        infos.sort();

        let mut improved = 0;

        for info in &mut infos {
            if info.backoff > 0 && time_since_improved < 2 * params.len() {
                time_since_improved += 1;
                info.backoff -= 1;
                println!("Backed off on parameter {} for {} more iterations", info.index, info.backoff);
            } else {
                let param = params[info.index];
                println!("Param {}, value {}, error = {}", info.index, param, best_error);

                if time_since_improved >= 2 * params.len() {
                    info.increment /= info.increment.abs();
                }

                params[info.index] += info.increment;

                let new_error = error(&params);

                println!("Sample {}, error {}", params[info.index], new_error);

                if new_error < best_error {
                    info.increment *= 2;
                    info.can_backoff = false;
                    info.backoff = 0;
                    info.prev_backoff = 0;

                    best_error = new_error;
                    time_since_improved = 0;
                    improved += 1;
                    println!("Changed param {}: {} -> {}", info.index, param, params[info.index]);
                } else {
                    params[info.index] = param;
                    time_since_improved += 1;

                    if info.increment.abs() == 1 {
                        if info.can_backoff {
                            info.prev_backoff += 2;
                            info.backoff = info.prev_backoff;
                            info.can_backoff = false;
                            println!("Backing off on parameter {} for {} iterations", info.index, info.backoff);
                        } else {
                            info.can_backoff = true;
                        }

                        info.increment *= -1;
                    } else {
                        info.increment = info.increment.signum();
                    }
                }

                println!();
            }
        }

        println!("Improved {} parameters.", improved);

        if time_since_improved >= 4 * params.len() {
            break;
        }
    }
}

use bitvec::{vec::BitVec, bitvec};

fn precompute(file: &str, positions: &[(f64, Board)], params: &EvalParams) -> Vec<BitVec> {
    println!("Begin precomputation");

    if let Ok(read) = File::open(file) {
        println!("Reading from cache file {}", file);
        let mut out: Vec<BitVec> = rmp_serde::decode::from_read(read).unwrap();

        return out;
    }

    let mut params = params.to_vec();

    let mut generator = MoveGenerator::empty();
    let mut pawn_tt = TT::with_len(16384);
    let mut evals = Vec::new();

    let mut params2 = EvalParams::from_vec(&params);

    for (_, board) in positions {
        evals.push(generator.eval_with_params(board.clone(), &mut pawn_tt, &params2));
    }

    let mut out = Vec::new();

    for i in 0..params.len() {
        let mut vec = BitVec::with_capacity(positions.len());
        let mut affected = 0;

        params[i] += 1;
        params2 = EvalParams::from_vec(&params);

        pawn_tt.clear();

        for (j, (_, board)) in positions.iter().enumerate() {
            let eval = generator.eval_with_params(board.clone(), &mut pawn_tt, &params2);

            vec.push(eval != evals[j]);

            if eval != evals[j] {
                affected += 1;
            }
        }

        println!("Param {} affects {} positions.", i, affected);

        params[i] -= 1;
        out.push(vec);
    }

    let mut write = File::create(file).unwrap();

    println!("Writing to cache file {}", file);
    rmp_serde::encode::write(&mut write, &out);

    out
}

struct ParamTuner {
    positions: Vec<(f64, Board)>,
    params: Vec<i32>,
    affected_positions: Vec<BitVec>,

    errors1: Vec<f64>,
    errors2: Vec<f64>,
    best_error: f64,

    generator: MoveGenerator,
    pawn_tt: TT,
}

fn win_prob(eval: i32) -> f64 {
    1. / (1. + 10f64.powf(-0.6773868 * eval as f64 / 400.))
}

fn eval_error(expected: f64, eval: i32) -> f64 {
    (expected - win_prob(eval)).powi(2)
}

impl ParamTuner {
    fn init_errors(&mut self) {
        self.pawn_tt.clear();

        let params = EvalParams::from_vec(&self.params);
        let mut total_error = 0.;

        for (i, (expected, board)) in self.positions.iter().enumerate() {
            let eval = self.generator.eval_with_params(board.clone(), &mut self.pawn_tt, &params);
            let error = eval_error(*expected, eval);

            self.errors1[i] = error;
            self.errors2[i] = error;

            total_error += error;
        }

        self.best_error = total_error / self.positions.len() as f64;
    }

    fn new(positions: Vec<(f64, Board)>, params: Vec<i32>, affected_positions: Vec<BitVec>) -> Self {
        let n_positions = positions.len();

        let mut out =
            Self {
                positions,
                params,
                affected_positions,

                errors1: vec![0.; n_positions],
                errors2: vec![0.; n_positions],
                best_error: 0.,

                generator: MoveGenerator::empty(),
                pawn_tt: TT::with_len(1024),
            };

        out.init_errors();

        out
    }

    fn try_step(&mut self, param: usize, incr: i32) -> bool {
        self.params[param] += incr;

        let params = EvalParams::from_vec(&self.params);
        let mut total_error = 0.;

        self.pawn_tt.clear();

        for (i, affects) in self.affected_positions[param].iter().enumerate() {
            let error;

            if *affects {
                let board = self.positions[i].1.clone();
                let eval = self.generator.eval_with_params(board, &mut self.pawn_tt, &params);

                error = eval_error(self.positions[i].0, eval);
            } else {
                error = self.errors1[i];
            }

            self.errors2[i] = error;
            total_error += error;
        }

        total_error /= self.positions.len() as f64;

        if total_error < self.best_error {
            self.best_error = total_error;
            self.errors1.copy_from_slice(&self.errors2);

            true
        } else {
            self.params[param] -= incr;

            false
        }
    }

    fn tune(&mut self) -> EvalParams {
        println!("Begin tuning");

        let mut increments = vec![1; self.params.len()];
        let mut can_stop = false;

        for i in 0.. {
            println!("vec!{:?}", self.params);
            println!("Begin iteration {}, error = {}", i, self.best_error);

            let mut improved = 0;

            for j in 0..self.params.len() {
                let param = self.params[j];
                println!("Param {}, value {}, error = {}", j, param, self.best_error);

                println!("Sampling {}", param + increments[j]);

                if self.try_step(j, increments[j]) {
                    println!("Changed param {}: {} -> {}", j, param, param + increments[j]);
                    increments[j] *= 2;
                    improved += 1;
                } else if increments[j].abs() == 1 {
                    increments[j] *= -1;
                } else {
                    increments[j] = increments[j].signum();
                }

                println!();
            }

            println!("Improved {} parameters.", improved);

            let mut all_1 = true;

            for x in &increments {
                if x.abs() != 1 {
                    all_1 = false;
                    break;
                }
            }

            if all_1 && improved == 0 {
                if can_stop {
                    break;
                } else {
                    can_stop = true;
                }
            } else {
                can_stop = false;
            }
        }

        EvalParams::from_vec(&self.params)
    }
}

pub fn tune(position_file: &str, cache_file: &str, params: &EvalParams) -> EvalParams {
    println!("Reading positions from {}", position_file);
    let positions = read_positions(position_file);
    println!("Read {} positions", positions.len());

    let affected_positions = precompute(cache_file, &positions, &params);

    let mut tuner = ParamTuner::new(positions, params.to_vec(), affected_positions);

    tuner.tune()

    // tune_abstract_backoff(&mut params, |p| {
        // let params = EvalParams::from_vec(p);

        // let mut error = 0.;
        // pawn_tt.clear();

        // for (expected, board) in &positions {
            // // println!("{}", board.to_fen(false));

            // let score = generator.eval_with_params(board.clone(), &mut pawn_tt, &params);
            // let win_prob = 1. / (1. + 10f64.powf(-0.6773868 * score as f64 / 400.));

            // // println!("{} {} {}", score, win_prob, expected);

            // error += (expected - win_prob).powi(2);
        // }

        // error / positions.len() as f64
    // });

    // EvalParams::from_vec(&params)
}
