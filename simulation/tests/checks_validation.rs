use fifo_ttt_simulation::checks::{check_line_invariant, check_v4_against, check_v6, run_checks, verify_reference_anchors, REFERENCE_CSV};
use fifo_ttt_simulation::config::{Cap, Condition, Player, Rule, K_VALUES};
use fifo_ttt_simulation::graph::{Graph, Terminal};
use fifo_ttt_simulation::retro::{retrograde, Solution};
use fifo_ttt_simulation::state::State;

fn solve(rule: Rule, kx: Cap, ko: Cap) -> (Condition, Graph, Solution) {
    let cond = Condition::new(rule, kx, ko, Player::X);
    let graph = Graph::enumerate(&cond);
    let solution = retrograde(&graph);
    (cond, graph, solution)
}

#[test]
fn classical_checks_cover_v1_through_v7_and_the_line_invariant() {
    let (cond, graph, solution) = solve(Rule::T3, Cap::Inf, Cap::Inf);
    let results = run_checks(&cond, &graph, &solution);

    for id in ["v1", "v2", "v4", "v5", "v6", "v7", "r41"] {
        let result = results
            .iter()
            .find(|result| result.id == id)
            .unwrap_or_else(|| panic!("missing check {id}"));
        assert!(result.passed, "{id}: {}", result.detail);
    }
}

#[test]
fn v3_distinguishes_t2_cap_two_from_t1_and_t3() {
    for rule in [Rule::T1, Rule::T3] {
        let (cond, graph, solution) = solve(rule, Cap::Finite(2), Cap::Finite(2));
        let result = run_checks(&cond, &graph, &solution)
            .into_iter()
            .find(|result| result.id == "v3")
            .expect("V-3 must apply to low caps");
        assert!(result.passed, "{rule}: {}", result.detail);
    }

    let (cond, graph, solution) = solve(Rule::T2, Cap::Finite(2), Cap::Finite(2));
    let result = run_checks(&cond, &graph, &solution)
        .into_iter()
        .find(|result| result.id == "v3")
        .expect("V-3 must check T2 cap two");
    assert!(result.passed, "{}", result.detail);
}

#[test]
fn reference_csv_is_required_and_malformed_rows_are_errors() {
    let (cond, graph, solution) = solve(Rule::T1, Cap::Inf, Cap::Inf);
    let anchors = verify_reference_anchors(REFERENCE_CSV, &cond, &graph, &solution).expect("the repository reference must parse");
    assert!(anchors.iter().all(|result| result.passed));

    let malformed = "target_id,value\nclassic.n_states,not-a-number\n";
    let error = verify_reference_anchors(malformed, &cond, &graph, &solution).expect_err("invalid numeric anchors must be rejected");
    assert!(error.contains("classic.n_states"));
}

#[test]
fn strengthened_v6_rejects_an_incorrect_distance() {
    let (_cond, graph, mut solution) = solve(Rule::T3, Cap::Finite(3), Cap::Finite(3));
    let root = graph.root as usize;
    solution.plies[root] = solution.plies[root].map(|plies| plies + 2);

    let result = check_v6(&graph, &solution);
    assert!(!result.passed);
    assert!(result.detail.contains("state 0"));
}

#[test]
fn v4_compares_against_the_independently_solved_canonical_graph() {
    let (cond, graph, solution) = solve(Rule::T1, Cap::Finite(2), Cap::Finite(2));
    let canonical_graph = Graph::enumerate_canonical(&cond);
    let mut canonical_solution = retrograde(&canonical_graph);
    canonical_solution.value[canonical_graph.root as usize] = 1;
    canonical_solution.plies[canonical_graph.root as usize] = Some(1);

    let result = check_v4_against(&graph, &solution, &canonical_graph, &canonical_solution);
    assert!(!result.passed);
    assert!(result.detail.contains("root"));
}

#[test]
fn line_invariant_rejects_a_line_in_an_ongoing_state() {
    let (cond, mut graph, _solution) = solve(Rule::T3, Cap::Finite(3), Cap::Finite(3));
    graph.states[graph.root as usize] = State::from_parts(&[0, 1, 2], &[], Player::O, cond.kx, cond.ko).unwrap();

    let result = check_line_invariant(&graph);
    assert!(!result.passed);
    assert!(result.detail.contains("state 0"));
}

#[test]
fn line_invariant_rejects_a_winning_terminal_without_a_winning_predecessor() {
    let (_cond, mut graph, _solution) = solve(Rule::T3, Cap::Finite(3), Cap::Finite(3));
    graph.terminal[graph.root as usize] = Terminal::MoverWon;

    let result = check_line_invariant(&graph);
    assert!(!result.passed);
    assert!(result.detail.contains("state 0"));
}

#[test]
fn every_unbounded_phase_one_condition_passes_its_applicable_checks() {
    for rule in [Rule::T1, Rule::T2, Rule::T3] {
        for kx in K_VALUES {
            for ko in K_VALUES {
                let (cond, graph, solution) = solve(rule, kx, ko);
                let failures: Vec<_> = run_checks(&cond, &graph, &solution).into_iter().filter(|result| !result.passed).collect();
                assert!(failures.is_empty(), "rule={rule}, kx={}, ko={}: {failures:?}", kx.label(), ko.label());
            }
        }
    }
}
