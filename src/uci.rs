// this is a submodule of search

use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};

use super::*;

#[derive(Clone, Debug)]
pub enum SearcherCommand {
    SetBoard(Board, Vec<Move>),
    SetDebug(bool),
    SetC960(bool),
    Search(Duration, u8),
    SearchPerft(usize, Arc<Mutex<Vec<Move>>>, Arc<AtomicU64>),
    Exit
}

pub use SearcherCommand::*;

impl Searcher {
    pub fn listen(&mut self) {
        let mut board = Board::from_fen(START_FEN);
        let mut moves = Vec::new();

        while let Ok(msg) = self.recv.recv() {
            match msg {
                SearchPerft(depth, moves, total) => {
                    if self.id == 0 {
                        // lock moves on the first thread so that we can clear the transpotition
                        // table without the other threads working
                        let _lock = moves.lock().unwrap();

                        self.tt.clear();
                    }

                    loop {
                        let mut lock = moves.lock().unwrap();

                        if let Some(mov) = lock.pop() {
                            std::mem::drop(lock);

                            let board2 = board.do_move(mov);
                            let res = self.perft(board2, depth - 1);
                            total.fetch_add(res, Ordering::Relaxed);

                            println!("{} {}", mov, res);
                        } else {
                            break;
                        }
                    }

                    if self.id == 0 {
                        self.tt.clear();
                    }
                }
                Search(time, d) => {
                    self.stop_time = Instant::now() + time;
                    self.prev_pos.clear();

                    let mut board2 = board.clone();

                    for m in &moves {
                        self.incr_prev_pos(board2.hash);
                        board2 = board2.do_move(*m);
                    }

                    self.history = [[[0usize; 64]; 64]; 2];

                    self.search(board2.clone(), d.min(1), d);
                },
                SetBoard(b, m) => {
                    board = b;
                    moves = m;
                    self.incr_time();
                },
                SetC960(b) => self.c960 = b,
                Exit => break,
                _ => {}
            }
        }
    }
}

struct ThreadPool {
    threads: Vec<JoinHandle<()>>,
    sends: Vec<Sender<SearcherCommand>>,
    stops: Vec<Sender<()>>,
    tt: TT,
    pawn_tt: TT,
    tasks: Vec<usize>,
}

impl ThreadPool {
    fn new(tt: TT, pawn_tt: TT, len: usize) -> Self {
        let mut out =
            ThreadPool {
                threads: Vec::new(),
                sends: Vec::new(),
                stops: Vec::new(),
                tt, 
                pawn_tt,
                tasks: Vec::new(),
            };

        out.add(len);

        out
    }

    fn add(&mut self, n: usize) {
        for _ in 0..n {
            let (send, recv) = channel();
            let (s_st, r_st) = channel();

            self.sends.push(send);
            self.stops.push(s_st);

            let tt = self.tt.clone();
            let pawn_tt = self.pawn_tt.clone();
            let id = self.threads.len();

            self.threads.push(
                thread::spawn(move || {
                    let mut searcher = Searcher::new(tt, pawn_tt, recv, r_st, id);

                    searcher.listen();
                })
            );

            self.tasks.push(0);
        }
    }

    fn remove(&mut self, n: usize) {
        for _ in 0..n {
            if !self.threads.is_empty() {
                let thread = self.threads.len() - 1;

                self.stops[thread].send(()).unwrap();
                self.send(thread, Exit);

                self.threads.pop().unwrap().join().unwrap();
                self.sends.pop().unwrap();
                self.stops.pop().unwrap();
                self.tasks.pop().unwrap();
            }
        }
    }

    fn set_size(&mut self, size: usize) {
        if size > self.threads.len() {
            self.add(size - self.threads.len());
        } else {
            self.remove(self.threads.len() - size);
        }
    }

    fn send(&mut self, thread: usize, cmd: SearcherCommand) {
        match cmd {
            Search(..) | SearchPerft(..) => {
                self.tasks[thread] += 1;
            }
            _ => {}
        }

        self.sends[thread].send(cmd).unwrap();
    }

    fn send_all(&mut self, cmd: SearcherCommand) {
        for t in 0..self.threads.len() {
            self.send(t, cmd.clone());
        }
    }

    fn stop(&mut self, thread: usize) {
        self.stops[thread].send(()).unwrap();
    }

    fn stop_all(&mut self) {
        for t in 0..self.threads.len() {
            self.stop(t);
        }
    }

    fn join(self) {
        for thread in self.threads.into_iter() {
            let _ = thread.join();
        }
    }
}

use std::io::{prelude::*, BufReader, BufRead};

pub fn ucimanager<T>(read: BufReader<T>) where T: Read {
    let mut tt = TT::with_len(62500);
    let     pawn_tt = TT::with_len(62500);

    let mut generator = MoveGenerator::empty();
    let     searcher = Searcher::new(tt.clone(), pawn_tt.clone(), channel().1, channel().1, 0);
    let mut threads = ThreadPool::new(tt.clone(), pawn_tt.clone(), 1);

    let mut lines = read.lines();
    let mut line = lines.next().unwrap().unwrap();
    let mut words = line.split_whitespace();

    let mut c960 = false;
    let mut board = Board::from_fen(START_FEN);
    let mut moves = Vec::new();

    'outer: loop {
        match words.next() {
            Some("isready") => println!("readyok"),
            Some("uci") => {
                println!("id name Bad Engine");
                println!("id author Nathan Koppel");
                println!();
                println!("option name Hash type spin default 1 min 1 max 1000000");
                println!("option name Threads type spin default 1 min 1 max 64");
                println!("option name Ponder type check default false");
                println!("option name UCI_Chess960 type check default false");
                println!("uciok");
            },
            Some("position") => {
                let pos = words.next().unwrap();

                if pos == "startpos" {
                    board = Board::from_fen(START_FEN);
                } else {
                    let mut s = String::new();

                    for _ in 0..6 {
                        s += words.next().unwrap();
                        s += " ";
                    }

                    board = Board::from_fen(&s);
                }

                let _ = words.next();

                moves = words.map(|w| Move::from_uci(w)).collect::<Vec<_>>();

                line = lines.next().unwrap().unwrap();
                words = line.split_whitespace();

                threads.send_all(SetBoard(board.clone(), moves.clone()));
            },
            Some("getposition") => {
                let board2 = moves.iter().fold(board.clone(), |b, m| b.do_move(*m));

                println!("info string {} 0 0", board2.to_fen(c960));
            }
            Some("domoves") => {
                moves.extend(words.map(|w| Move::from_uci(w)));

                line = lines.next().unwrap().unwrap();
                words = line.split_whitespace();

                threads.send_all(SetBoard(board.clone(), moves.clone()));
            }
            Some("dobestmove") => {
                let reps = words
                    .next()
                    .unwrap_or("1")
                    .parse::<usize>()
                    .unwrap_or(1);

                let board2 = moves.iter().fold(board.clone(), |b, m| b.do_move(*m));

                for _ in 0..reps {
                    if let Some(mov) = searcher.get_best_move(&board2) {
                        moves.push(mov);
                    } else {
                        break;
                    }
                }

                threads.send_all(SetBoard(board.clone(), moves.clone()));
            }
            Some("waitonsearch") => {
                threads.send_all(Exit);
                threads.join();

                threads = ThreadPool::new(tt.clone(), pawn_tt.clone(), 1);
            }
            Some("setoption") => {
                let mut val = false;
                let mut name = String::new();
                let mut value = String::new();

                words.next().unwrap();

                for w in words {
                    if w == "value" {
                        val = true;
                    } else if val {
                        value += w;
                        value += " ";
                    } else {
                        name += w;
                        name += " ";
                    }
                }

                match name.trim() {
                    "Hash" => {
                        if let Ok(n) = value.trim().parse::<usize>() {
                            unsafe {
                                tt.resize(n * 62500)
                            }
                        }
                    }
                    "Threads" => {
                        if let Ok(n @ 0..=64) = value.trim().parse::<usize>() {
                            threads.set_size(n)
                        }
                    }
                    "UCI_Chess960" => {
                        if let Ok(c) = value.trim().parse::<bool>() {
                            threads.send_all(SetC960(c));
                            c960 = c;
                        }
                    }
                    _ => {}
                }
                line = lines.next().unwrap().unwrap();
                words = line.split_whitespace();
            }
            Some("go") => {
                let mut depth = 255;
                let mut time = Duration::from_secs(3155760000);
                let mut movetime = Duration::from_secs(0);
                let mut inc = Duration::from_secs(0);
                let mut movestogo = 30;

                while let Some(w) = words.next() {
                    match w {
                        "depth" => {
                            if let Some(w) = words.next() {
                                if let Ok(d) = w.parse::<u8>() {
                                    depth = d
                                }
                            }
                        }
                        "movestogo" => {
                            if let Some(w) = words.next() {
                                if let Ok(m) = w.parse::<u32>() {
                                    movestogo = m
                                }
                            }
                        }
                        "wtime" | "btime" => {
                            if board.black ^ (moves.len() & 1 != 0) == (w == "btime") {
                                if let Some(w) = words.next() {
                                    if let Ok(t) = w.parse::<u64>() {
                                        time = Duration::from_millis(t);
                                    }
                                }
                            }
                        }
                        "winc" | "binc" => {
                            if board.black ^ (moves.len() & 1 != 0) == (w == "binc") {
                                if let Some(w) = words.next() {
                                    if let Ok(t) = w.parse::<u64>() {
                                        inc = Duration::from_millis(t);
                                    }
                                }
                            }
                        }
                        "movetime" => {
                            if let Some(w) = words.next() {
                                if let Ok(t) = w.parse::<u64>() {
                                    movetime = Duration::from_millis(t);
                                }
                            }
                        }
                        "perft" => {
                            let depth = 
                                if let Some(w) = words.next() {
                                    if let Ok(d) = w.parse::<usize>() {
                                        d
                                    } else {
                                        continue;
                                    }
                                } else {
                                    continue;
                                };

                            generator.set_board(board.clone());
                            generator.gen_moves();

                            let moves = generator.moves.drain(..).map(|b| board.get_move(&b, c960)).collect::<Vec<_>>();
                            let moves = Arc::new(Mutex::new(moves));
                            let total = Arc::new(AtomicU64::new(0));

                            threads.send_all(SearchPerft(depth, moves, total));

                            continue 'outer;
                        }
                        _ => {}
                    }
                }

                if movetime.as_nanos() == 0 {
                    let margin = Duration::from_millis(3);
                    movetime = (time / movestogo).max(inc);

                    if time.saturating_sub(movetime) < margin {
                        movetime = time - margin
                    }
                }

                threads.stop_all();
                threads.send_all(Search(movetime, depth));
            }
            Some("stop") => threads.stop_all(),
            Some("quit") => break,
            Some(_) => {},
            None => {
                if let Some(Ok(l)) = lines.next() {
                    line = l;
                    words = line.split_whitespace();
                } else {
                    threads.send_all(Exit);
                    threads.join();
                    break;
                }
            }
        }
    }
}

pub fn perftmanager(ttsize: usize, threads: usize, board: Board, depth: usize) {
    let tt = TT::with_len(ttsize);
    let pawn_tt = TT::with_len(1);

    let mut threads = ThreadPool::new(tt.clone(), pawn_tt.clone(), threads);

    let mut generator = MoveGenerator::new(board.clone());
    generator.gen_moves();

    let moves: Vec<_> =
        generator.moves.iter().map(|b| board.get_move(&b, false)).collect();

    let moves = Arc::new(Mutex::new(moves));
    let total = Arc::new(AtomicU64::new(0));

    threads.send_all(SetBoard(board.clone(), Vec::new()));
    threads.send_all(SearchPerft(depth, moves.clone(), total.clone()));
    threads.send_all(Exit);

    threads.join();

    println!();
    println!("{}", total.load(Ordering::Relaxed));
}
