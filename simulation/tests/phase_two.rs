use fifo_ttt_simulation::checks::{check_v11, check_v8, check_v9};
use fifo_ttt_simulation::config::{sweep_specs, Cap, Condition, Player, Rule};
use fifo_ttt_simulation::graph::Graph;
use fifo_ttt_simulation::iterate::value_iteration;
use fifo_ttt_simulation::naive::naive_negamax;
use fifo_ttt_simulation::retro::retrograde;
use fifo_ttt_simulation::state::State;
use fifo_ttt_simulation::world::{EndReason, GameEnd, GameResult};

#[test]
fn bounded_iteration_matches_unbounded_distance_for_every_state() {
    let cond = Condition::new(Rule::T3, Cap::Finite(3), Cap::Finite(2), Player::X);
    let graph = Graph::enumerate(&cond);
    let unbounded = retrograde(&graph);

    for horizon in [0, 1, 2, 5, 20] {
        let bounded = value_iteration(&graph.states, &cond, Some(horizon));
        assert!(check_v8(&graph, &unbounded, &bounded, horizon).passed);
        for index in 0..graph.states.len() {
            let expected = if unbounded.plies[index].is_some_and(|plies| plies <= horizon) {
                unbounded.value[index]
            } else {
                0
            };
            assert_eq!(bounded.value[index], expected, "horizon={horizon}, state={index}");
        }
    }
}

#[test]
fn o_first_is_the_negated_role_swapped_x_first_game() {
    for rule in [Rule::T1, Rule::T2, Rule::T3] {
        let result = check_v9(rule, Cap::Finite(2), Cap::Finite(3));
        assert!(result.passed, "{}", result.detail);
    }
}

#[test]
fn naive_classical_tree_has_the_known_node_count() {
    for rule in [Rule::T1, Rule::T2, Rule::T3] {
        let cond = Condition::new(rule, Cap::Inf, Cap::Inf, Player::X);
        let result = naive_negamax(&cond, 549_946);
        assert_eq!(result.value, Some(0));
        assert_eq!(result.nodes, 549_946);
        assert!(!result.capped);
    }
}

#[test]
fn naive_cap_never_reports_a_partial_value() {
    let cond = Condition::new(Rule::T3, Cap::Inf, Cap::Inf, Player::X);
    let result = naive_negamax(&cond, 10);
    assert_eq!(result.value, None);
    assert_eq!(result.nodes, 10);
    assert!(result.capped);
}

#[test]
fn all_sweep_has_three_rules_and_five_sensitivity_settings() {
    let specs = sweep_specs();
    assert_eq!(specs.len(), 15);
    for rule in [Rule::T1, Rule::T2, Rule::T3] {
        let for_rule: Vec<_> = specs.iter().filter(|spec| spec.rule == rule).collect();
        assert_eq!(for_rule.len(), 5);
        assert!(for_rule.iter().any(|spec| spec.first == Player::X && spec.horizon.is_none()));
        for horizon in [20, 50, 100] {
            assert!(for_rule.iter().any(|spec| spec.first == Player::X && spec.horizon == Some(horizon)));
        }
        assert!(for_rule.iter().any(|spec| spec.first == Player::O && spec.horizon.is_none()));
    }
}

#[test]
fn v11_accepts_draw_cutoffs_but_rejects_late_solved_wins() {
    let draw = GameEnd {
        result: GameResult::Draw,
        plies: 200,
        reason: EndReason::Cap,
        repeat_to: None,
        final_state: State::initial(Player::X),
    };
    assert!(check_v11(0, None, Player::X, &draw).passed);

    let late_win = GameEnd {
        result: GameResult::XWin,
        plies: 8,
        reason: EndReason::Line,
        repeat_to: None,
        final_state: State::initial(Player::O),
    };
    assert!(!check_v11(1, Some(7), Player::X, &late_win).passed);
}
