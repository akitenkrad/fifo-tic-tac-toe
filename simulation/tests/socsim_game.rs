use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use fifo_ttt_simulation::config::{Cap, Condition, Player, Rule};
use fifo_ttt_simulation::graph::Graph;
use fifo_ttt_simulation::mechanisms::{ApplyMove, ChooseMove, Referee};
use fifo_ttt_simulation::policy::{Observation, OptimalPolicy, Policy, ScriptedPolicy, UniformRandomPolicy};
use fifo_ttt_simulation::retro::retrograde;
use fifo_ttt_simulation::rules::{legal_moves, step};
use fifo_ttt_simulation::simulation::{play_game, PlayOptions};
use fifo_ttt_simulation::state::State;
use fifo_ttt_simulation::world::{EndReason, GameResult, GameWorld};
use rand::RngCore;
use socsim_core::{Mechanism, NullRecorder, Phase, Recorder, SimRng, WorldState};
use socsim_engine::{SequentialScheduler, SimulationBuilder};

fn condition(rule: Rule, cap: Cap) -> Condition {
    Condition::new(rule, cap, cap, Player::X)
}

#[derive(Clone, Default)]
struct EventRecorder {
    events: Rc<RefCell<Vec<(u64, String, serde_json::Value)>>>,
}

impl Recorder for EventRecorder {
    fn record_metric(&mut self, _t: u64, _key: &str, _value: f64) {}

    fn record_event(&mut self, t: u64, kind: &str, payload: serde_json::Value) {
        self.events.borrow_mut().push((t, kind.to_owned(), payload));
    }
}

#[test]
fn world_uses_solver_state_and_fixed_agent_ids() {
    let cond = condition(Rule::T3, Cap::Finite(3));
    let world = GameWorld::new(cond, "pv.kx3_ko3", 42, true);

    assert_eq!(world.state, State::initial(Player::X));
    assert_eq!(world.to_move(), Player::X);
    assert_eq!(world.agent_ids(), vec![socsim_core::AgentId(0), socsim_core::AgentId(1)]);
    assert_eq!(world.clock().t(), 0);
    assert!(world.end().is_none());
    assert_eq!(world.seen.get(&world.state), Some(&0));
}

#[test]
fn mechanisms_register_only_the_required_phases() {
    let choose = ChooseMove::new(Box::new(ScriptedPolicy::new([0])), Box::new(ScriptedPolicy::new([1])));
    assert_eq!(choose.phases(), &[Phase::Decision]);
    assert_eq!(ApplyMove.phases(), &[Phase::Interaction]);
    assert_eq!(Referee.phases(), &[Phase::PostStep]);
}

#[test]
fn scripted_and_random_policies_choose_legal_moves() {
    let cond = condition(Rule::T3, Cap::Finite(3));
    let state = State::from_parts(&[0], &[4], Player::X, cond.kx, cond.ko).unwrap();
    let legal = legal_moves(&state, &cond);
    let obs = Observation { state, cond, legal };

    let mut scripted = ScriptedPolicy::new([8]);
    assert_eq!(scripted.choose(&obs, &mut SimRng::from_seed(1)), 8);

    for seed in 0..64 {
        let mut random = UniformRandomPolicy;
        let selected = random.choose(&obs, &mut SimRng::from_seed(seed));
        assert!(obs.legal.contains(&selected));
    }
}

#[test]
fn optimal_policy_uses_value_and_distance_without_consuming_rng() {
    let cond = condition(Rule::T3, Cap::Inf);
    let root = State::initial(Player::X);
    let (child_zero, _) = step(&root, 0, &cond).unwrap();
    let (child_one, _) = step(&root, 1, &cond).unwrap();
    let values = HashMap::from([
        (root.packed(), (1, Some(3))),
        (child_zero.packed(), (-1, Some(4))),
        (child_one.packed(), (-1, Some(2))),
    ]);
    let obs = Observation {
        state: root,
        cond,
        legal: vec![0, 1],
    };
    let mut policy = OptimalPolicy::new(values);
    let mut used = SimRng::from_seed(99);
    let mut untouched = SimRng::from_seed(99);

    assert_eq!(policy.choose(&obs, &mut used), 1);
    assert_eq!(used.next_u64(), untouched.next_u64());
}

#[test]
fn optimal_policy_breaks_decisive_ties_by_the_smallest_cell() {
    let cond = condition(Rule::T3, Cap::Inf);
    let root = State::initial(Player::X);
    let child_zero = step(&root, 0, &cond).unwrap().0;
    let child_one = step(&root, 1, &cond).unwrap().0;
    let values = HashMap::from([(child_zero.packed(), (-1, Some(2))), (child_one.packed(), (-1, Some(2)))]);
    let obs = Observation {
        state: root,
        cond,
        legal: vec![0, 1],
    };
    let mut policy = OptimalPolicy::new(values);

    assert_eq!(policy.choose(&obs, &mut SimRng::from_seed(0)), 0);
}

#[test]
fn optimal_policy_can_be_built_from_the_retrograde_solution() {
    let cond = condition(Rule::T3, Cap::Inf);
    let graph = Graph::enumerate(&cond);
    let solution = retrograde(&graph);
    let mut policy = OptimalPolicy::from_graph(&graph, &solution);
    let root = graph.states[graph.root as usize];
    let obs = Observation {
        state: root,
        cond,
        legal: legal_moves(&root, &cond),
    };

    assert_eq!(policy.choose(&obs, &mut SimRng::from_seed(0)), 0);
}

#[test]
fn choose_move_rejects_an_illegal_policy_result() {
    let cond = condition(Rule::T3, Cap::Inf);
    let result = play_game(
        cond,
        Box::new(ScriptedPolicy::new([9])),
        Box::new(ScriptedPolicy::new([0])),
        Box::new(NullRecorder),
        0,
        PlayOptions::new("invalid", 1, false),
    );

    let error = result.unwrap_err();
    assert!(error.contains("illegal move 9"));
}

#[test]
fn first_ply_is_recorded_at_t_one() {
    let recorder = EventRecorder::default();
    let events = recorder.events.clone();
    let end = play_game(
        condition(Rule::T3, Cap::Inf),
        Box::new(ScriptedPolicy::new([0])),
        Box::new(ScriptedPolicy::new([1])),
        Box::new(recorder),
        0,
        PlayOptions::new("pv.kxinf_koinf", 1, false),
    )
    .unwrap();

    assert_eq!(end.reason, EndReason::Cap);
    let events = events.borrow();
    let ply = events.iter().find(|(_, kind, _)| kind == "ply").unwrap();
    assert_eq!(ply.0, 1);
    assert_eq!(ply.2["unit_id"], "pv.kxinf_koinf");
    assert_eq!(ply.2["player"], "x");
    assert_eq!(ply.2["cell"], 0);
}

#[test]
fn ply_event_reports_a_removed_cell_even_when_it_is_immediately_reused() {
    let recorder = EventRecorder::default();
    let events = recorder.events.clone();
    play_game(
        condition(Rule::T1, Cap::Finite(1)),
        Box::new(ScriptedPolicy::new([4, 4])),
        Box::new(ScriptedPolicy::new([0])),
        Box::new(recorder),
        0,
        PlayOptions::new("reuse", 3, false),
    )
    .unwrap();

    let events = events.borrow();
    let last_ply = events.iter().rev().find(|(_, kind, _)| kind == "ply").unwrap();
    assert_eq!(last_ply.0, 3);
    assert_eq!(last_ply.2["cell"], 4);
    assert_eq!(last_ply.2["removed"], 4);
}

#[test]
fn referee_stops_on_a_winning_line() {
    let end = play_game(
        condition(Rule::T3, Cap::Inf),
        Box::new(ScriptedPolicy::new([0, 1, 2])),
        Box::new(ScriptedPolicy::new([3, 4])),
        Box::new(NullRecorder),
        0,
        PlayOptions::new("line", 5, true),
    )
    .unwrap();

    assert_eq!(end.reason, EndReason::Line);
    assert_eq!(end.result, GameResult::XWin);
    assert_eq!(end.plies, 5);
}

#[test]
fn referee_stops_when_the_board_has_no_legal_move() {
    let end = play_game(
        condition(Rule::T3, Cap::Inf),
        Box::new(ScriptedPolicy::new([0, 2, 3, 7, 8])),
        Box::new(ScriptedPolicy::new([1, 4, 5, 6])),
        Box::new(NullRecorder),
        0,
        PlayOptions::new("no-move", 9, true),
    )
    .unwrap();

    assert_eq!(end.reason, EndReason::NoMove);
    assert_eq!(end.result, GameResult::Draw);
    assert_eq!(end.plies, 9);
}

#[test]
fn referee_stops_on_repeat_and_reports_the_return_ply() {
    let end = play_game(
        condition(Rule::T1, Cap::Finite(1)),
        Box::new(ScriptedPolicy::new([0, 2, 0])),
        Box::new(ScriptedPolicy::new([1, 3, 1])),
        Box::new(NullRecorder),
        0,
        PlayOptions::new("repeat", 6, true),
    )
    .unwrap();

    assert_eq!(end.reason, EndReason::Repeat);
    assert_eq!(end.result, GameResult::Draw);
    assert_eq!(end.plies, 6);
    assert_eq!(end.repeat_to, Some(2));
}

#[test]
fn referee_stops_at_the_ply_cap() {
    let end = play_game(
        condition(Rule::T3, Cap::Inf),
        Box::new(ScriptedPolicy::new([0])),
        Box::new(ScriptedPolicy::new([1])),
        Box::new(NullRecorder),
        0,
        PlayOptions::new("cap", 1, false),
    )
    .unwrap();

    assert_eq!(end.reason, EndReason::Cap);
    assert_eq!(end.result, GameResult::Draw);
    assert_eq!(end.plies, 1);
}

#[test]
fn socsim_one_step_matches_rules_for_all_small_reachable_states() {
    let cond = condition(Rule::T1, Cap::Finite(1));
    let mut queue = VecDeque::from([State::initial(Player::X)]);
    let mut seen = HashSet::from([State::initial(Player::X)]);

    while let Some(state) = queue.pop_front() {
        for cell in legal_moves(&state, &cond) {
            let expected = step(&state, cell, &cond).unwrap().0;
            let world = GameWorld::from_state(cond, state, "shared-step", 1, false);
            let x: Box<dyn Policy> = if state.to_move() == Player::X {
                Box::new(ScriptedPolicy::new([cell]))
            } else {
                Box::new(ScriptedPolicy::new([0]))
            };
            let o: Box<dyn Policy> = if state.to_move() == Player::O {
                Box::new(ScriptedPolicy::new([cell]))
            } else {
                Box::new(ScriptedPolicy::new([0]))
            };
            let mut simulation = SimulationBuilder::new(world)
                .scheduler(Box::new(SequentialScheduler))
                .seed(0)
                .add_mechanism(Box::new(ChooseMove::new(x, o)))
                .add_mechanism(Box::new(ApplyMove))
                .add_mechanism(Box::new(Referee))
                .build();

            simulation.step().unwrap();
            assert_eq!(simulation.world().state, expected);

            if seen.insert(expected) {
                queue.push_back(expected);
            }
        }
    }

    assert!(seen.len() > 100);
}
