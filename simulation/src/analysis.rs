use std::collections::BTreeSet;

use crate::config::Player;
use crate::graph::{Edge, Graph};
use crate::retro::Solution;
use crate::rules::Outcome;
use crate::symmetry::{stabilizer, D4};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoveClass {
    pub class_id: u32,
    pub cells: Vec<u8>,
    pub value_after_x: i8,
    pub plies_after: Option<u32>,
}

pub fn best_moves(graph: &Graph, solution: &Solution, root: u32) -> Vec<MoveClass> {
    let root_index = root as usize;
    let root_value = solution.value[root_index];
    let root_plies = solution.plies[root_index];
    let edges = graph.successors(root);
    let best: BTreeSet<u8> = edges
        .iter()
        .filter(|edge| {
            let (value, total_plies) = mover_evaluation(edge, solution);
            value == root_value && (root_value == 0 || total_plies == root_plies)
        })
        .map(|edge| edge.cell)
        .collect();
    let stabilizer = stabilizer(&graph.states[root_index]);
    let mut remaining = best.clone();
    let mut classes = Vec::new();

    while let Some(representative) = remaining.first().copied() {
        let orbit: BTreeSet<u8> = stabilizer
            .iter()
            .map(|symmetry| D4[*symmetry][usize::from(representative)])
            .filter(|cell| best.contains(cell))
            .collect();
        for cell in &orbit {
            remaining.remove(cell);
        }
        let edge = edges
            .iter()
            .find(|edge| edge.cell == representative)
            .expect("an orbit representative must have an edge");
        let value_after_x = value_after_x(graph, root, edge, solution);
        let plies_after = if edge.outcome == Outcome::MoverWins {
            Some(0)
        } else {
            solution.plies[edge.to as usize]
        };
        classes.push(MoveClass {
            class_id: classes.len() as u32,
            cells: orbit.into_iter().collect(),
            value_after_x,
            plies_after,
        });
    }
    classes
}

fn mover_evaluation(edge: &Edge, solution: &Solution) -> (i8, Option<u32>) {
    if edge.outcome == Outcome::MoverWins {
        (1, Some(1))
    } else {
        (-solution.value[edge.to as usize], solution.plies[edge.to as usize].map(|plies| plies + 1))
    }
}

fn value_after_x(graph: &Graph, root: u32, edge: &Edge, solution: &Solution) -> i8 {
    if edge.outcome == Outcome::MoverWins {
        if graph.states[root as usize].to_move() == Player::X {
            1
        } else {
            -1
        }
    } else if graph.states[edge.to as usize].to_move() == Player::X {
        solution.value[edge.to as usize]
    } else {
        -solution.value[edge.to as usize]
    }
}
