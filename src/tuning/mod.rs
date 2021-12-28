mod positions_from_games;

pub use positions_from_games::positions_from_games;

use crate::gen_moves::*;
use crate::eval::*;
use crate::board::*;
use crate::tt::*;

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

            // pawn_weight  : 100,
            // knight_weight: vec[11],
            // bishop_weight: vec[12],
            // rook_weight  : vec[13],
            // queen_weight : vec[14],
            // king_weight  : CHECKMATE,

            pawn_weight  : 0,
            knight_weight: 0,
            bishop_weight: 0,
            rook_weight  : 0,
            queen_weight : 0,
            king_weight  : CHECKMATE,

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
}

fn tune_abstract<F>(params: &mut Vec<i32>, mut error: F)
    where F: FnMut(&[i32]) -> f64
{
    println!("Begin tuning");

    let mut best_error = error(&params);

    for i in 0.. {
        println!("vec!{:?}", params);
        println!("Begin iteration {}, error = {}", i, best_error);

        let mut improved = 0;

        for j in 0..params.len() {
            println!();
            println!("Param {}, value {}, error = {}", j, params[j], best_error);

            let param = params[j];

            let sample2 = best_error;

            params[j] -= 1;
            let sample1 = error(&params);
            println!("Sample {}, error {}", param - 1, sample1);
            params[j] += 2;
            let sample3 = error(&params);
            println!("Sample {}, error {}", param + 1, sample2);

            let slope1 = sample2 - sample1;
            let slope2 = sample3 - sample2;

            let slope = (slope1 + slope2) / 2.;
            let concav = slope2 - slope1;

            let min = (param as f64 - slope / concav + 0.5) as i32;

            let sample4;

            if min >= param - 1 && min <= param + 1 {
                sample4 = sample2;
            } else {
                params[j] = min;
                sample4 = error(&params);
                println!("Sample {}, error {}", min, sample4);
            }

            best_error = sample1.min(sample2.min(sample3.min(sample4)));

            if best_error == sample2 {
                params[j] = param;
            } else if best_error == sample1 {
                improved += 1;
                params[j] = param - 1;
                println!("Changed param {}: {} -> {}", j, param, param - 1);
            } else if best_error == sample3 {
                improved += 1;
                params[j] = param + 1;
                println!("Changed param {}: {} -> {}", j, param, param + 1);
            } else {
                improved += 1;
                params[j] = min;
                println!("Changed param {}: {} -> {}", j, param, min);
            }
        }

        println!("Improved {} parameters.", improved);

        if improved == 0 {
            println!("Did not improve; exiting");
            break;
        }
    }
}

pub fn tune(position_file: &str, params: &EvalParams) -> EvalParams {
    println!("Reading positions from {}", position_file);
    let positions = read_positions(position_file);
    println!("Read {} positions", positions.len());

    let mut generator = MoveGenerator::empty();
    let mut pawn_tt = TT::with_len(16384);

    // let mut params = params.to_vec();
    let mut params = vec![-2, -6, -5, 2, -5, -49, 0, 1, 2, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 32, 27, 28, 19, 22, 28, 32, 26, 29, 23, 32, 27, 29, 33, 29, 24, 23, 29, 17, 25, 27, 21, 31, 26, 40, 36, 30, 31, 28, 28, 50, 46, 78, 57, 55, 70, 74, 73, 70, 63, 94, 121, 97, 83, 83, 134, 114, 94, 0, 0, 0, 0, 0, 0, 0, 0, 34, 16, 32, 35, 42, 19, 28, 28, 52, 50, 39, 36, 31, 44, 49, -5, 29, 49, 41, 51, 45, 39, 42, 20, 40, 51, 49, 47, 41, 41, 48, 40, 62, 41, 66, 51, 65, 50, 45, 41, 35, 42, 47, 61, 60, 45, 54, 51, 43, 75, 50, 48, 48, 64, 45, 29, 23, 76, 51, 52, 52, 56, 38, 68, 46, -14, 30, 22, 30, 35, 36, 59, 60, 48, 47, 43, 40, 45, 40, 21, 37, 57, 44, 42, 43, 43, 41, 31, 32, 30, 39, 46, 42, 39, 37, 48, 42, 39, 35, 49, 50, 48, 41, 29, 58, 44, 41, 40, 44, 39, 50, 36, 7, 43, 29, 39, 47, 48, 44, 50, 97, 58, 36, 52, 36, 62, 37, 58, 39, 103, 60, 86, 87, 83, 60, 75, 61, 84, 90, 89, 86, 91, 67, 98, 119, 91, 87, 82, 84, 84, 84, 78, 86, 90, 89, 85, 86, 94, 84, 85, 90, 78, 108, 97, 78, 87, 79, 92, 108, 118, 110, 114, 97, 113, 92, 98, 109, 131, 133, 107, 110, 114, 102, 99, 134, 143, 132, 114, 76, 132, 108, 125, -99, -41, -100, -89, -99, -39, -87, -122, -92, -89, -86, -91, -91, -91, -86, -98, -92, -88, -78, -80, -81, -83, -85, -89, -108, -82, -82, -75, -69, -76, -82, -101, -87, -79, -70, -78, -75, -74, -77, -74, -103, -88, -75, -72, -69, -81, -79, -70, -112, -102, -104, -78, -94, -97, -85, -35, -38, -99, -84, -115, -95, -81, -53, -34, 29, 51, 44, 47, 47, 45, 43, 40, 34, 44, 48, 50, 50, 51, 45, 49, 48, 33, 47, 46, 43, 48, 41, 41, 51, 46, 53, 51, 52, 56, 55, 54, 53, 45, 60, 47, 50, 55, 52, 57, 57, 60, 57, 59, 57, 58, 62, 60, 54, 58, 61, 61, 63, 61, 61, 56, 52, 63, 52, 51, 58, 58, 60, 60, 0, 0, 0, 0, 0, 0, 0, 0, 82, 88, 110, 84, 101, 102, 108, 104, 60, 71, 54, 56, 64, 74, 74, 68, 31, 43, 30, 34, 34, 33, 39, 33, 17, 30, 19, 29, 29, 26, 28, 31, 24, 28, 30, 31, 23, 27, 22, 30, 23, 33, 29, 27, 15, 28, 26, 25, 0, 0, 0, 0, 0, 0, 0, 0, 28, 65, 60, 57, 51, 73, 42, 45, 33, 45, 51, 49, 60, 61, 36, 37, 44, 79, 40, 50, 54, 46, 43, 44, 43, 40, 61, 53, 52, 49, 45, 33, 56, 34, 49, 43, 51, 51, 48, 38, 38, 42, 45, 40, 44, 38, 33, 23, 23, 30, 46, 39, 34, 26, 34, 19, -29, 23, 9, 13, 31, 37, 29, 4, 42, 48, 42, 43, 37, 50, 35, 57, 4, 34, 50, 37, 29, 34, 42, 37, 34, 43, 33, 46, 42, 37, 48, 39, 45, 27, 40, 45, 52, 39, 40, 20, 48, 26, 35, 39, 38, 44, 38, 32, 27, 51, 42, 35, 38, 40, 33, 26, 86, 43, 48, 42, 31, 30, 37, 49, 28, 27, 25, 26, 31, 27, 80, 31, 143, 136, 128, 112, 88, 117, 122, 116, 112, 104, 113, 94, 96, 91, 81, 93, 114, 107, 106, 109, 105, 98, 85, 99, 101, 87, 99, 92, 80, 88, 69, 91, 83, 93, 81, 79, 69, 84, 79, 76, 90, 91, 82, 82, 77, 87, 81, 82, 96, 86, 84, 82, 90, 88, 86, 104, 93, 30, 68, 86, 87, 89, 91, 78, -80, -63, -72, -58, -87, -62, -35, -22, -82, -111, -100, -70, -73, -91, -90, -70, -93, -84, -75, -70, -60, -79, -74, -82, -89, -81, -76, -74, -68, -63, -73, -67, -95, -85, -78, -81, -74, -72, -80, -86, -91, -84, -82, -83, -87, -78, -77, -89, -88, -85, -87, -88, -93, -86, -77, -88, -93, -35, -97, -84, -89, -38, -88, -87, 53, 60, 50, 48, 53, 60, 55, 56, 49, 59, 66, 60, 58, 59, 61, 59, 60, 56, 60, 55, 51, 57, 58, 56, 54, 56, 56, 51, 49, 53, 54, 59, 49, 53, 54, 50, 50, 58, 58, 51, 50, 53, 54, 49, 48, 48, 47, 52, 44, 48, 56, 51, 48, 51, 46, 50, 32, 43, 47, 51, 48, 49, 49, 43];

    tune_abstract(&mut params, |p| {
        let params = EvalParams::from_vec(p);

        let mut error = 0.;
        pawn_tt.clear();

        for (expected, board) in &positions {
            let score = generator.eval_with_params(board.clone(), &mut pawn_tt, &params);
            let win_prob = 1. / (1. + 10f64.powf(-score as f64 / 400.));

            // println!("{} {} {} {:?}", score, win_prob, expected, board);

            error += (expected - win_prob).powi(2);
        }

        error / positions.len() as f64
    });

    EvalParams::from_vec(&params)
}
