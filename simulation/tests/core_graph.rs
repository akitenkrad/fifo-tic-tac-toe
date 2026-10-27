use std::collections::HashSet;

use fifo_ttt_simulation::config::{Cap, Condition, Player, Rule};
use fifo_ttt_simulation::graph::Graph;
use fifo_ttt_simulation::retro::retrograde;
use fifo_ttt_simulation::rules::Outcome;
use fifo_ttt_simulation::symmetry::canonical;
use fifo_ttt_simulation::symmetry::stabilizer;

fn classic(rule: Rule) -> Condition {
    Condition::new(rule, Cap::Inf, Cap::Inf, Player::X)
}

#[test]
fn classical_enumeration_matches_all_three_anchors() {
    for rule in [Rule::T1, Rule::T2, Rule::T3] {
        let graph = Graph::enumerate(&classic(rule));
        let canonical_states: HashSet<_> = graph.states.iter().map(canonical).collect();

        assert_eq!(graph.states.len(), 5_478, "rule={rule}");
        assert_eq!(canonical_states.len(), 765, "rule={rule}");
        assert_eq!(graph.n_games(), Some(255_168), "rule={rule}");
    }
}

#[test]
fn graph_csr_contains_forward_and_reverse_edges() {
    let graph = Graph::enumerate(&classic(Rule::T3));
    let forward_count: usize = (0..graph.states.len()).map(|index| graph.successors(index as u32).len()).sum();
    let reverse_count: usize = (0..graph.states.len()).map(|index| graph.predecessors(index as u32).len()).sum();

    assert_eq!(forward_count, reverse_count);
    for edge in graph.successors(graph.root) {
        assert_eq!(edge.outcome, Outcome::Continue);
        assert!(graph
            .predecessors(edge.to)
            .iter()
            .any(|back| back.to == graph.root && back.cell == edge.cell));
    }
}

#[test]
fn low_caps_distinguish_t2_line_detection() {
    let no_line_t1 = Graph::enumerate(&Condition::new(Rule::T1, Cap::Finite(2), Cap::Finite(2), Player::X));
    let line_t2 = Graph::enumerate(&Condition::new(Rule::T2, Cap::Finite(2), Cap::Finite(2), Player::X));
    let no_line_t3 = Graph::enumerate(&Condition::new(Rule::T3, Cap::Finite(2), Cap::Finite(2), Player::X));

    assert!(!no_line_t1.edges.iter().any(|edge| edge.outcome == Outcome::MoverWins));
    assert!(line_t2.edges.iter().any(|edge| edge.outcome == Outcome::MoverWins));
    assert!(!no_line_t3.edges.iter().any(|edge| edge.outcome == Outcome::MoverWins));
}

#[test]
fn canonical_quotient_is_solved_and_expands_to_full_counts() {
    for rule in [Rule::T1, Rule::T2, Rule::T3] {
        let cond = classic(rule);
        let full = Graph::enumerate(&cond);
        let quotient = Graph::enumerate_canonical(&cond);
        let full_solution = retrograde(&full);
        let quotient_solution = retrograde(&quotient);
        let expanded_states: usize = quotient.states.iter().map(|state| 8 / stabilizer(state).len()).sum();

        assert_eq!(quotient.states.len(), 765, "rule={rule}");
        assert_eq!(expanded_states, 5_478, "rule={rule}");
        assert_eq!(
            quotient_solution.value[quotient.root as usize], full_solution.value[full.root as usize],
            "rule={rule}"
        );
        assert_eq!(
            quotient_solution.plies[quotient.root as usize], full_solution.plies[full.root as usize],
            "rule={rule}"
        );
    }
}
