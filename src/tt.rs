use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TT {
    table: Arc<Vec<u64>>,
}

impl TT {
    pub fn new() -> Self {
        Self {
            table: Arc::new(Vec::new())
        }
    }

    pub fn with_len(len: usize) -> Self {
        Self {
            table: Arc::new(vec![0; len * 2])
        }
    }

    pub fn len(&self) -> usize {
        self.table.len() / 2
    }

    pub unsafe fn resize(&mut self, len: usize) {
        // ub if table is being read or written to while this occurrs
        Arc::get_mut_unchecked(&mut self.table).resize(len * 2, 0)
    }

    pub fn resize_safe(&mut self, len: usize) -> bool {
        if let Some(t) = Arc::get_mut(&mut self.table) {
            t.resize(len * 2, 0);

            true
        } else {
            false
        }
    }

    pub fn read(&self, hash: u64) -> Option<u64> {
        if self.table.is_empty() {
            return None;
        }
        let ind = hash as usize % self.len() * 2;
        let h = self.table[ind    ];
        let d = self.table[ind + 1];

        if hash == h ^ d {
            Some(d)
        } else {
            None
        }
    }

    pub fn force_read(&self, hash: u64) -> (u64, u64) {
        let ind = hash as usize % self.len() * 2;
        let h = self.table[ind    ];
        let d = self.table[ind + 1];

        (h ^ d, d)
    }

    pub fn write(&mut self, hash: u64, data: u64) {
        unsafe {
            let t = Arc::get_mut_unchecked(&mut self.table);

            if t.is_empty() {
                return;
            }

            let ind = hash as usize % (t.len() / 2) * 2;
            t[ind    ] = hash ^ data;
            t[ind + 1] = data;
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            Arc::get_mut_unchecked(&mut self.table).fill(0)
        }
    }
}

#[allow(unused_imports)]
use test::Bencher;

#[allow(unused_imports)]
use std::sync::Barrier;
#[allow(unused_imports)]
use std::thread;

#[test]
fn t_resize() {
    let mut tt = TT::new();

    unsafe{tt.resize(20);}
    assert_eq!(tt.len(), 20);

    assert!(tt.resize_safe(10));
    assert_eq!(tt.len(), 10);

    let _tt2 = tt.clone();

    unsafe{tt.resize(20);}
    assert_eq!(tt.len(), 20);

    assert!(!tt.resize_safe(10));
    assert_eq!(tt.len(), 20);
}

#[test]
fn t_write() {
    let mut handles = Vec::with_capacity(16);
    let tt = TT::with_len(16);

    for i in 0..16 {
        let mut t = tt.clone();

        handles.push(thread::spawn(move|| {
            t.write(0, i);
            t.write(i, i);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("{:?}", tt);

    assert_eq!(tt.read(0).unwrap() >> 4, 0);
    assert_eq!(tt.read(15), Some(15));
}

#[bench]
fn b_read(b: &mut Bencher) {
    let mut tt = TT::with_len(1);

    tt.write(0, 1234);

    b.iter(|| tt.read(0));
}

#[bench]
fn b_write(b: &mut Bencher) {
    let mut tt = TT::new();

    tt.resize_safe(1);

    b.iter(|| tt.write(0, 1234));
}
