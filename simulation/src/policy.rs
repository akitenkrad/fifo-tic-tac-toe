use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};

use rand::Rng;
use socsim_core::SimRng;

use crate::config::Condition;
use crate::graph::Graph;
use crate::retro::Solution;
use crate::rules::{step, Outcome};
use crate::state::State;

#[derive(Clone, Debug)]
pub struct Observation {
    pub state: State,
    pub cond: Condition,
    pub legal: Vec<u8>,
}

pub trait Policy {
    fn choose(&mut self, obs: &Observation, rng: &mut SimRng) -> u8;
}

pub struct OptimalPolicy {
    evaluations: HashMap<u64, (i8, Option<u32>)>,
}

impl OptimalPolicy {
    pub fn new(evaluations: HashMap<u64, (i8, Option<u32>)>) -> Self {
        Self { evaluations }
    }

    pub fn evaluations(&self) -> &HashMap<u64, (i8, Option<u32>)> {
        &self.evaluations
    }

    pub fn from_graph(graph: &Graph, solution: &Solution) -> Self {
        assert_eq!(graph.states.len(), solution.value.len(), "solution values must cover every graph state");
        assert_eq!(
            graph.states.len(),
            solution.plies.len(),
            "solution distances must cover every graph state"
        );
        let evaluations = graph
            .states
            .iter()
            .zip(solution.value.iter().zip(&solution.plies))
            .map(|(state, (value, plies))| (state.packed(), (*value, *plies)))
            .collect();
        Self::new(evaluations)
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    cell: u8,
    value: i8,
    plies: Option<u32>,
}

impl Candidate {
    fn compare(self, other: Self) -> Ordering {
        self.value
            .cmp(&other.value)
            .then_with(|| match self.value {
                1 => other.plies.unwrap_or(u32::MAX).cmp(&self.plies.unwrap_or(u32::MAX)),
                -1 => self.plies.unwrap_or(0).cmp(&other.plies.unwrap_or(0)),
                _ => Ordering::Equal,
            })
            .then_with(|| other.cell.cmp(&self.cell))
    }
}

impl Policy for OptimalPolicy {
    fn choose(&mut self, obs: &Observation, _rng: &mut SimRng) -> u8 {
        let best = obs
            .legal
            .iter()
            .filter_map(|cell| {
                let (next, outcome) = step(&obs.state, *cell, &obs.cond).ok()?;
                let (value, plies) = if outcome == Outcome::MoverWins {
                    (1, Some(1))
                } else {
                    let (child_value, child_plies) = self.evaluations.get(&next.packed())?;
                    (-*child_value, child_plies.map(|distance| distance + 1))
                };
                Some(Candidate { cell: *cell, value, plies })
            })
            .max_by(|left, right| left.compare(*right));

        best.map_or_else(|| obs.legal.iter().copied().min().unwrap_or(u8::MAX), |candidate| candidate.cell)
    }
}

#[derive(Default)]
pub struct UniformRandomPolicy;

impl Policy for UniformRandomPolicy {
    fn choose(&mut self, obs: &Observation, rng: &mut SimRng) -> u8 {
        if obs.legal.is_empty() {
            u8::MAX
        } else {
            obs.legal[rng.gen_range(0..obs.legal.len())]
        }
    }
}

pub struct ScriptedPolicy {
    moves: VecDeque<u8>,
}

impl ScriptedPolicy {
    pub fn new(moves: impl IntoIterator<Item = u8>) -> Self {
        Self {
            moves: moves.into_iter().collect(),
        }
    }
}

impl Policy for ScriptedPolicy {
    fn choose(&mut self, _obs: &Observation, _rng: &mut SimRng) -> u8 {
        self.moves.pop_front().unwrap_or(u8::MAX)
    }
}
