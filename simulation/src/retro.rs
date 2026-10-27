use std::collections::VecDeque;

use crate::graph::{Graph, Terminal};
use crate::rules::Outcome;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Solution {
    pub value: Vec<i8>,
    pub plies: Vec<Option<u32>>,
}

pub fn retrograde(graph: &Graph) -> Solution {
    let n = graph.states.len();
    let mut value = vec![0_i8; n];
    let mut plies = vec![None; n];
    let mut unresolved: Vec<usize> = (0..n)
        .map(|state| {
            graph
                .successors(state as u32)
                .iter()
                .filter(|edge| edge.outcome == Outcome::Continue)
                .count()
        })
        .collect();
    let mut greatest_winning_child = vec![0_u32; n];
    let mut queue = VecDeque::new();

    for state in 0..n {
        if graph.terminal[state] == Terminal::MoverWon {
            value[state] = -1;
            plies[state] = Some(0);
            queue.push_back(state as u32);
        }
    }
    for state in 0..n {
        if value[state] == 0 && graph.successors(state as u32).iter().any(|edge| edge.outcome == Outcome::MoverWins) {
            value[state] = 1;
            plies[state] = Some(1);
            queue.push_back(state as u32);
        }
    }

    while let Some(solved) = queue.pop_front() {
        let solved_index = solved as usize;
        let distance = plies[solved_index].expect("decisive states have a distance");
        for incoming in graph.predecessors(solved) {
            if incoming.outcome != Outcome::Continue {
                continue;
            }
            let predecessor = incoming.to as usize;
            if value[predecessor] != 0 {
                continue;
            }
            if value[solved_index] == -1 {
                value[predecessor] = 1;
                plies[predecessor] = Some(distance + 1);
                queue.push_back(predecessor as u32);
            } else {
                unresolved[predecessor] -= 1;
                greatest_winning_child[predecessor] = greatest_winning_child[predecessor].max(distance);
                if unresolved[predecessor] == 0 {
                    value[predecessor] = -1;
                    plies[predecessor] = Some(greatest_winning_child[predecessor] + 1);
                    queue.push_back(predecessor as u32);
                }
            }
        }
    }

    Solution { value, plies }
}
