use std::collections::{HashMap, VecDeque};

use crate::config::Condition;
use crate::rules::{legal_moves, step, Outcome};
use crate::state::State;
use crate::symmetry::canonical;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edge {
    pub to: u32,
    pub cell: u8,
    pub outcome: Outcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Terminal {
    Ongoing,
    MoverWon,
    NoMoves,
}

#[derive(Clone, Debug)]
pub struct Graph {
    pub cond: Condition,
    pub root: u32,
    pub states: Vec<State>,
    pub terminal: Vec<Terminal>,
    pub offsets: Vec<u32>,
    pub edges: Vec<Edge>,
    pub reverse_offsets: Vec<u32>,
    pub reverse_edges: Vec<Edge>,
    ids: HashMap<u64, u32>,
}

impl Graph {
    pub fn enumerate(cond: &Condition) -> Self {
        Self::enumerate_with_symmetry(cond, false)
    }

    pub fn enumerate_canonical(cond: &Condition) -> Self {
        Self::enumerate_with_symmetry(cond, true)
    }

    fn enumerate_with_symmetry(cond: &Condition, canonicalize: bool) -> Self {
        let initial = State::initial(cond.first);
        let root_state = if canonicalize { canonical(&initial) } else { initial };
        let mut states = vec![root_state];
        let mut ids = HashMap::from([(root_state.packed(), 0)]);
        let mut adjacency = vec![Vec::new()];
        let mut continuing = vec![true];
        let mut queued = vec![true];
        let mut queue = VecDeque::from([0_u32]);

        while let Some(from) = queue.pop_front() {
            let state = states[from as usize];
            let mut outgoing = Vec::new();
            for cell in legal_moves(&state, cond) {
                let (raw_next, outcome) = step(&state, cell, cond).expect("a move returned by legal_moves must be accepted by step");
                let next = if canonicalize { canonical(&raw_next) } else { raw_next };
                let to = if let Some(existing) = ids.get(&next.packed()) {
                    *existing
                } else {
                    let id = u32::try_from(states.len()).expect("state count must fit in u32");
                    states.push(next);
                    ids.insert(next.packed(), id);
                    adjacency.push(Vec::new());
                    continuing.push(false);
                    queued.push(false);
                    id
                };
                if outcome == Outcome::Continue {
                    continuing[to as usize] = true;
                    if !queued[to as usize] {
                        queued[to as usize] = true;
                        queue.push_back(to);
                    }
                }
                outgoing.push(Edge { to, cell, outcome });
            }
            adjacency[from as usize] = outgoing;
        }

        let terminal = adjacency
            .iter()
            .enumerate()
            .map(|(index, edges)| {
                if !continuing[index] {
                    Terminal::MoverWon
                } else if edges.is_empty() {
                    Terminal::NoMoves
                } else {
                    Terminal::Ongoing
                }
            })
            .collect();
        let (offsets, edges) = flatten(adjacency);
        let mut reverse = vec![Vec::new(); states.len()];
        for from in 0..states.len() {
            for edge in &edges[offsets[from] as usize..offsets[from + 1] as usize] {
                reverse[edge.to as usize].push(Edge {
                    to: from as u32,
                    cell: edge.cell,
                    outcome: edge.outcome,
                });
            }
        }
        let (reverse_offsets, reverse_edges) = flatten(reverse);

        Self {
            cond: *cond,
            root: 0,
            states,
            terminal,
            offsets,
            edges,
            reverse_offsets,
            reverse_edges,
            ids,
        }
    }

    pub fn state_id(&self, state: &State) -> Option<u32> {
        self.ids.get(&state.packed()).copied()
    }

    pub fn successors(&self, state: u32) -> &[Edge] {
        let index = state as usize;
        &self.edges[self.offsets[index] as usize..self.offsets[index + 1] as usize]
    }

    pub fn predecessors(&self, state: u32) -> &[Edge] {
        let index = state as usize;
        &self.reverse_edges[self.reverse_offsets[index] as usize..self.reverse_offsets[index + 1] as usize]
    }

    pub fn n_games(&self) -> Option<u64> {
        let mut marks = vec![0_u8; self.states.len()];
        let mut memo = vec![None; self.states.len()];
        self.count_games(self.root, &mut marks, &mut memo)
    }

    fn count_games(&self, state: u32, marks: &mut [u8], memo: &mut [Option<u64>]) -> Option<u64> {
        let index = state as usize;
        if marks[index] == 1 {
            return None;
        }
        if marks[index] == 2 {
            return memo[index];
        }
        marks[index] = 1;
        let successors = self.successors(state);
        let mut total = if successors.is_empty() { 1_u64 } else { 0_u64 };
        for edge in successors {
            let paths = if edge.outcome == Outcome::MoverWins {
                1
            } else {
                self.count_games(edge.to, marks, memo)?
            };
            total = total.checked_add(paths)?;
        }
        marks[index] = 2;
        memo[index] = Some(total);
        Some(total)
    }
}

fn flatten(adjacency: Vec<Vec<Edge>>) -> (Vec<u32>, Vec<Edge>) {
    let mut offsets = Vec::with_capacity(adjacency.len() + 1);
    let edge_count: usize = adjacency.iter().map(Vec::len).sum();
    let mut edges = Vec::with_capacity(edge_count);
    offsets.push(0);
    for mut outgoing in adjacency {
        edges.append(&mut outgoing);
        offsets.push(u32::try_from(edges.len()).expect("edge count must fit in u32"));
    }
    (offsets, edges)
}
