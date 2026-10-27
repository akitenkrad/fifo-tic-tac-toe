use fifo_ttt_simulation::analysis::best_moves;
use fifo_ttt_simulation::config::{Cap, Condition, Player, Rule};
use fifo_ttt_simulation::graph::Graph;
use fifo_ttt_simulation::retro::retrograde;
use fifo_ttt_simulation::state::State;
use fifo_ttt_simulation::symmetry::{canonical, stabilizer, transform};

#[test]
fn canonicalization_preserves_queue_order_under_each_transform() {
    let state = State::from_parts(&[0, 5, 7], &[1, 3], Player::O, Cap::Finite(3), Cap::Finite(2)).unwrap();
    let canonical_state = canonical(&state);

    assert!((0..8).any(|symmetry| transform(&state, symmetry) == canonical_state));
    assert_eq!(canonical(&canonical_state), canonical_state);
    assert_eq!(canonical_state.to_move(), Player::O);
    assert_eq!(canonical_state.cells_in_order(Player::X).len(), 3);
    assert_eq!(canonical_state.cells_in_order(Player::O).len(), 2);
}

#[test]
fn empty_board_has_the_full_d4_stabilizer() {
    assert_eq!(stabilizer(&State::initial(Player::X)), (0..8).collect::<Vec<_>>());
}

#[test]
fn classical_initial_best_moves_form_corner_edge_and_center_orbits() {
    let cond = Condition::new(Rule::T3, Cap::Inf, Cap::Inf, Player::X);
    let graph = Graph::enumerate(&cond);
    let solution = retrograde(&graph);
    let classes = best_moves(&graph, &solution, graph.root);
    let cells: Vec<_> = classes.iter().map(|class| class.cells.clone()).collect();

    assert_eq!(cells, vec![vec![0, 2, 6, 8], vec![1, 3, 5, 7], vec![4]]);
    assert!(classes.iter().all(|class| class.value_after_x == 0));
    assert!(classes.iter().all(|class| class.plies_after.is_none()));
}
