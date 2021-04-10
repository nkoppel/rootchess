use std::thread;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Arc, Mutex};

use crate::gen_tables::*;
use crate::board::*;
use crate::gen_moves::*;
use crate::moves::*;
use crate::tt::*;
use crate::search::*;

enum WorkerMessage {
    SetBoard(Board),
    SetDebug(bool),
    SetC960(bool),
    Search,
    SearchDepth(usize),
    SearchNodes(usize),
    SearchPerft(usize, Arc<Mutex<Vec<Move>>>),
    Exit
}

use WorkerMessage::*;

fn worker(recv: Receiver<WorkerMessage>,
          stop: Receiver<()>,
          id: usize,
          tt: TT,
          idle: Sender<usize>) 
{
    let mut board = Board::from_fen(START_FEN);
    let mut c960 = false;
    let mut searcher = Searcher::new(tt, false);

    while let Ok(msg) = recv.recv() {
        match msg {
            SearchPerft(depth, mut moves) => {
                let mut total = 0;
                loop {
                    let mov = {
                        if let Some(m) = moves.lock().unwrap().pop() {
                            m
                        } else {
                            break;
                        }
                    };
                    let board2 = board.do_move(mov);
                    let res = searcher.perft(board2, depth - 1);
                    total += res;

                    println!("{} {}", mov, res);
                }
                idle.send(total as usize).unwrap();
            },
            SetBoard(b) => board = b,
            SetC960(c) => c960 = c,
            Exit => break,
            _ => {}
        }
    }
}

pub fn perftmanager(ttsize: usize, threads: usize, board: Board, depth: usize) {
    let mut sends = Vec::new();
    let mut stops = Vec::new();
    let (s_idle, idle) = channel();

    let mut tt = TT::with_len(ttsize);

    for i in 0..threads {
        let (s_snd, r_snd) = channel();
        let (s_stp, r_stp) = channel();

        sends.push(s_snd);
        stops.push(s_stp);

        let tt = tt.clone();
        let s_idle = s_idle.clone();

        thread::spawn(move ||
            worker(r_snd, r_stp, i, tt, s_idle));
    }

    let mut generator = MoveGenerator::new(board.clone());
    generator.gen_moves();

    let moves: Vec<_> =
        generator.moves.iter().map(|b| board.get_move(&b, false)).collect();

    let moves = Arc::new(Mutex::new(moves));

    for i in 0..threads {
        sends[i].send(SetBoard(board.clone())).unwrap();
        sends[i].send(SearchPerft(depth, moves.clone())).unwrap();
    }

    let mut total = 0;

    for _ in 0..threads {
        total += idle.recv().unwrap();
    }

    for i in 0..threads {
        sends[i].send(Exit).unwrap();
    }

    println!();
    println!("{}", total);
}
