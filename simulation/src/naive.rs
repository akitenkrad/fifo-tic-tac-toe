use std::collections::HashSet;
use std::hash::{BuildHasherDefault, Hasher};
use std::time::{Duration, Instant};

use crate::config::Condition;
use crate::rules::{legal_moves, step, Outcome};
use crate::state::State;

const PROGRESS_INTERVAL: Duration = Duration::from_secs(30);
const PROGRESS_POLL_NODES: u64 = 65_536;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NaiveResult {
    pub value: Option<i8>,
    pub nodes: u64,
    pub capped: bool,
}

struct Frame {
    state: State,
    moves: u16,
    best: i8,
}

#[derive(Default)]
struct StateHasher(u64);

impl Hasher for StateHasher {
    fn finish(&self) -> u64 {
        let mut value = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut value = 0_u64;
        for (index, byte) in bytes.iter().take(8).enumerate() {
            value |= u64::from(*byte) << (index * 8);
        }
        self.0 = value;
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

impl Frame {
    fn new(state: State, cond: &Condition) -> Self {
        let moves = legal_moves(&state, cond).into_iter().fold(0_u16, |mask, cell| mask | (1 << cell));
        Self { state, moves, best: -1 }
    }
}

pub fn naive_negamax(cond: &Condition, node_cap: u64) -> NaiveResult {
    naive_negamax_with_progress(cond, node_cap, |_| {})
}

pub fn naive_negamax_with_progress(cond: &Condition, node_cap: u64, mut report: impl FnMut(u64)) -> NaiveResult {
    if node_cap == 0 {
        return NaiveResult {
            value: None,
            nodes: 0,
            capped: true,
        };
    }

    let root = State::initial(cond.first);
    let root_frame = Frame::new(root, cond);
    if root_frame.moves == 0 {
        return NaiveResult {
            value: Some(0),
            nodes: 1,
            capped: false,
        };
    }

    let mut stack = vec![root_frame];
    let mut path: HashSet<u64, BuildHasherDefault<StateHasher>> = HashSet::default();
    path.insert(root.packed());
    let mut nodes = 1_u64;
    let mut next_progress_poll = PROGRESS_POLL_NODES;
    let mut last_report = Instant::now();

    loop {
        let frame_index = stack.len() - 1;
        if stack[frame_index].moves == 0 {
            let completed = stack.pop().expect("the active frame must exist");
            path.remove(&completed.state.packed());
            if let Some(parent) = stack.last_mut() {
                parent.best = parent.best.max(-completed.best);
                continue;
            }
            return NaiveResult {
                value: Some(completed.best),
                nodes,
                capped: false,
            };
        }

        let cell = stack[frame_index].moves.trailing_zeros() as u8;
        stack[frame_index].moves &= !(1 << cell);
        if nodes == node_cap {
            return NaiveResult {
                value: None,
                nodes,
                capped: true,
            };
        }
        nodes += 1;

        if nodes >= next_progress_poll {
            next_progress_poll = nodes.saturating_add(PROGRESS_POLL_NODES);
            if last_report.elapsed() >= PROGRESS_INTERVAL {
                report(nodes);
                last_report = Instant::now();
            }
        }

        let state = stack[frame_index].state;
        let (next, outcome) = step(&state, cell, cond).expect("a move from legal_moves must be accepted");
        if outcome == Outcome::MoverWins {
            stack[frame_index].best = stack[frame_index].best.max(1);
            continue;
        }
        if path.contains(&next.packed()) {
            stack[frame_index].best = stack[frame_index].best.max(0);
            continue;
        }

        let child = Frame::new(next, cond);
        if child.moves == 0 {
            stack[frame_index].best = stack[frame_index].best.max(0);
            continue;
        }
        path.insert(next.packed());
        stack.push(child);
    }
}
