use std::collections::{HashMap, VecDeque};

use crate::config::Condition;
use crate::retro::Solution;
use crate::rules::{legal_moves, step, Outcome};
use crate::state::State;
use crate::symmetry::canonical;

pub fn value_iteration(states: &[State], cond: &Condition, max_iter: Option<u32>) -> Solution {
    if states.is_empty() {
        return Solution {
            value: Vec::new(),
            plies: Vec::new(),
        };
    }
    let ids: HashMap<_, _> = states.iter().enumerate().map(|(index, state)| (state.packed(), index)).collect();
    let mut continuing = vec![false; states.len()];
    continuing[0] = true;
    let mut queue = VecDeque::from([0_usize]);
    while let Some(index) = queue.pop_front() {
        for cell in legal_moves(&states[index], cond) {
            let (next, outcome) = step(&states[index], cell, cond).expect("a legal move must be accepted during value iteration");
            if outcome == Outcome::Continue {
                let next_index = state_index(&ids, &next);
                if !continuing[next_index] {
                    continuing[next_index] = true;
                    queue.push_back(next_index);
                }
            }
        }
    }

    let mut value: Vec<i8> = continuing.iter().map(|is_continuing| if *is_continuing { 0 } else { -1 }).collect();
    let mut plies: Vec<Option<u32>> = continuing
        .iter()
        .map(|is_continuing| if *is_continuing { None } else { Some(0) })
        .collect();
    let mut iteration = 0_u32;

    loop {
        if max_iter.is_some_and(|limit| iteration >= limit) {
            break;
        }
        iteration += 1;
        let previous = value.clone();
        let mut changed = false;
        for (index, state) in states.iter().enumerate() {
            if !continuing[index] {
                continue;
            }
            let legal = legal_moves(state, cond);
            if legal.is_empty() {
                continue;
            }
            let next_value = legal
                .into_iter()
                .map(|cell| {
                    let (next, outcome) = step(state, cell, cond).expect("a legal move must be accepted during value iteration");
                    if outcome == Outcome::MoverWins {
                        1
                    } else {
                        let next_index = state_index(&ids, &next);
                        -previous[next_index]
                    }
                })
                .max()
                .expect("a nonempty move list has a maximum");
            if next_value != previous[index] {
                value[index] = next_value;
                changed = true;
                if next_value != 0 && plies[index].is_none() {
                    plies[index] = Some(iteration);
                }
            }
        }
        if !changed {
            break;
        }
    }

    Solution { value, plies }
}

fn state_index(ids: &HashMap<u64, usize>, state: &State) -> usize {
    ids.get(&state.packed())
        .or_else(|| ids.get(&canonical(state).packed()))
        .copied()
        .expect("the enumerated state list must contain every successor")
}
