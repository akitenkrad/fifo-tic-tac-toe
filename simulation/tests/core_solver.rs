use fifo_ttt_simulation::config::{Cap, Condition, Player, Rule};
use fifo_ttt_simulation::graph::Graph;
use fifo_ttt_simulation::iterate::value_iteration;
use fifo_ttt_simulation::retro::retrograde;
use fifo_ttt_simulation::rules::Outcome;

fn solve(rule: Rule, kx: Cap, ko: Cap) -> (Graph, fifo_ttt_simulation::retro::Solution) {
    let cond = Condition::new(rule, kx, ko, Player::X);
    let graph = Graph::enumerate(&cond);
    let solution = retrograde(&graph);
    (graph, solution)
}

#[test]
fn classical_root_is_a_draw_for_every_rule() {
    for rule in [Rule::T1, Rule::T2, Rule::T3] {
        let (graph, solution) = solve(rule, Cap::Inf, Cap::Inf);
        assert_eq!(solution.value[graph.root as usize], 0, "rule={rule}");
        assert_eq!(solution.plies[graph.root as usize], None, "rule={rule}");
    }
}

#[test]
fn independent_value_iteration_matches_retrograde_values_and_plies() {
    for rule in [Rule::T1, Rule::T2, Rule::T3] {
        let (graph, retro) = solve(rule, Cap::Finite(3), Cap::Finite(3));
        let iterated = value_iteration(&graph.states, &graph.cond, None);
        assert_eq!(iterated.value, retro.value, "values for rule={rule}");
        assert_eq!(iterated.plies, retro.plies, "plies for rule={rule}");
    }
}

#[test]
fn value_iteration_can_solve_the_canonical_quotient() {
    let cond = Condition::new(Rule::T3, Cap::Finite(3), Cap::Finite(3), Player::X);
    let graph = Graph::enumerate_canonical(&cond);
    assert_eq!(value_iteration(&graph.states, &cond, None), retrograde(&graph));
}

#[test]
fn retrograde_plies_obey_minimax_distance_on_both_sides() {
    let (graph, solution) = solve(Rule::T3, Cap::Finite(3), Cap::Finite(3));
    for state in 0..graph.states.len() as u32 {
        let value = solution.value[state as usize];
        let plies = solution.plies[state as usize];
        let successors = graph.successors(state);
        match (value, plies) {
            (1, Some(n)) => {
                let losing_distances: Vec<_> = successors
                    .iter()
                    .filter_map(|edge| {
                        if edge.outcome == Outcome::MoverWins {
                            Some(0)
                        } else if solution.value[edge.to as usize] == -1 {
                            solution.plies[edge.to as usize]
                        } else {
                            None
                        }
                    })
                    .collect();
                assert_eq!(losing_distances.iter().min().copied(), Some(n - 1));
            }
            (-1, Some(n)) => {
                let distances: Vec<_> = successors.iter().map(|edge| solution.plies[edge.to as usize].unwrap()).collect();
                if !distances.is_empty() {
                    assert!(successors
                        .iter()
                        .all(|edge| { edge.outcome == Outcome::Continue && solution.value[edge.to as usize] == 1 }));
                    assert_eq!(distances.iter().max().copied(), Some(n - 1));
                }
            }
            (0, None) => {
                assert!(!successors
                    .iter()
                    .any(|edge| { edge.outcome == Outcome::MoverWins || solution.value[edge.to as usize] == -1 }));
                assert!(successors.is_empty() || successors.iter().any(|edge| solution.value[edge.to as usize] == 0));
            }
            other => panic!("invalid solution entry: {other:?}"),
        }
    }
}

#[test]
fn rules_without_three_retained_cells_have_no_winning_edges() {
    for rule in [Rule::T1, Rule::T3] {
        for kx in [Cap::Finite(1), Cap::Finite(2)] {
            for ko in [Cap::Finite(1), Cap::Finite(2)] {
                let (graph, solution) = solve(rule, kx, ko);
                assert!(graph.edges.iter().all(|edge| edge.outcome == Outcome::Continue));
                assert!(solution.value.iter().all(|value| *value == 0));
            }
        }
    }
    let (graph, solution) = solve(Rule::T2, Cap::Finite(1), Cap::Finite(1));
    assert!(graph.edges.iter().all(|edge| edge.outcome == Outcome::Continue));
    assert!(solution.value.iter().all(|value| *value == 0));
}
