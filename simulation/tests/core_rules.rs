use fifo_ttt_simulation::config::{Cap, Condition, Player, Rule};
use fifo_ttt_simulation::rules::{legal_moves, removed_cell, step, Outcome};
use fifo_ttt_simulation::state::{has_line, State};

fn condition(rule: Rule, kx: Cap, ko: Cap) -> Condition {
    Condition::new(rule, kx, ko, Player::X)
}

#[test]
fn finite_queues_preserve_oldest_to_newest_order() {
    let cond = condition(Rule::T3, Cap::Finite(3), Cap::Finite(3));
    let mut state = State::initial(Player::X);
    for cell in [0, 3, 1, 4] {
        state = step(&state, cell, &cond).unwrap().0;
    }

    assert_eq!(state.cells_in_order(Player::X), vec![0, 1]);
    assert_eq!(state.cells_in_order(Player::O), vec![3, 4]);
    assert_eq!(state.to_move(), Player::X);
}

#[test]
fn t1_allows_reusing_the_cell_removed_before_the_move() {
    let cond = condition(Rule::T1, Cap::Finite(1), Cap::Finite(1));
    let state = State::from_parts(&[4], &[0], Player::X, cond.kx, cond.ko).unwrap();

    assert!(legal_moves(&state, &cond).contains(&4));
    let (next, outcome) = step(&state, 4, &cond).unwrap();
    assert_eq!(outcome, Outcome::Continue);
    assert_eq!(next.cells_in_order(Player::X), vec![4]);
}

#[test]
fn t2_detects_a_line_before_removing_the_oldest_piece() {
    let cond = condition(Rule::T2, Cap::Finite(2), Cap::Finite(2));
    let state = State::from_parts(&[0, 1], &[3, 4], Player::X, cond.kx, cond.ko).unwrap();

    let (next, outcome) = step(&state, 2, &cond).unwrap();
    assert_eq!(outcome, Outcome::MoverWins);
    assert_eq!(next.cells_in_order(Player::X), vec![1, 2]);
    assert!(!has_line(next.cells_of(Player::X)));
}

#[test]
fn t3_does_not_count_a_line_containing_the_removed_piece() {
    let cond = condition(Rule::T3, Cap::Finite(2), Cap::Finite(2));
    let state = State::from_parts(&[0, 1], &[3, 4], Player::X, cond.kx, cond.ko).unwrap();

    let (next, outcome) = step(&state, 2, &cond).unwrap();
    assert_eq!(outcome, Outcome::Continue);
    assert_eq!(next.cells_in_order(Player::X), vec![1, 2]);
}

#[test]
fn infinite_caps_store_cells_as_an_orderless_mask() {
    let cond = condition(Rule::T3, Cap::Inf, Cap::Inf);
    let a = State::from_parts(&[0, 8, 4], &[], Player::O, cond.kx, cond.ko).unwrap();
    let b = State::from_parts(&[4, 0, 8], &[], Player::O, cond.kx, cond.ko).unwrap();

    assert_eq!(a, b);
    assert_eq!(a.cells_of(Player::X), (1 << 0) | (1 << 4) | (1 << 8));
}

#[test]
fn removed_cell_reports_the_rule_owned_queue_change() {
    let t1 = condition(Rule::T1, Cap::Finite(1), Cap::Finite(1));
    let state = State::from_parts(&[4], &[0], Player::X, t1.kx, t1.ko).unwrap();
    assert_eq!(removed_cell(&state, 4, &t1), Some(4));

    let t3 = condition(Rule::T3, Cap::Finite(2), Cap::Finite(2));
    let state = State::from_parts(&[0, 1], &[3, 4], Player::X, t3.kx, t3.ko).unwrap();
    assert_eq!(removed_cell(&state, 2, &t3), Some(0));

    let inf = condition(Rule::T2, Cap::Inf, Cap::Inf);
    let state = State::initial(Player::X);
    assert_eq!(removed_cell(&state, 0, &inf), None);
}

#[test]
fn full_board_move_availability_depends_on_removal_order() {
    let t1 = condition(Rule::T1, Cap::Finite(4), Cap::Inf);
    let state = State::from_parts(&[0, 1, 3, 5], &[2, 4, 6, 7, 8], Player::X, t1.kx, t1.ko).unwrap();
    assert_eq!(legal_moves(&state, &t1), vec![0]);

    for rule in [Rule::T2, Rule::T3] {
        let cond = condition(rule, Cap::Finite(4), Cap::Inf);
        assert!(legal_moves(&state, &cond).is_empty());
    }
}

#[test]
fn finite_queue_order_is_part_of_the_packed_state() {
    let a = State::from_parts(&[0, 1], &[4], Player::X, Cap::Finite(2), Cap::Finite(2)).unwrap();
    let b = State::from_parts(&[1, 0], &[4], Player::X, Cap::Finite(2), Cap::Finite(2)).unwrap();
    assert_ne!(a, b);
}
