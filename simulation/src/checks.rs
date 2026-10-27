use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use socsim_core::NullRecorder;

use crate::config::{Cap, Condition, Player, Rule};
use crate::graph::{Graph, Terminal};
use crate::iterate::value_iteration;
use crate::policy::OptimalPolicy;
use crate::retro::{retrograde, Solution};
use crate::rules::Outcome;
use crate::simulation::{play_game, PlayOptions};
use crate::state::has_line;
use crate::symmetry::{canonical, stabilizer};
use crate::world::{EndReason, GameResult};

pub const REFERENCE_CSV: &str = include_str!("../../reference.csv");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckResult {
    pub id: String,
    pub passed: bool,
    pub detail: String,
}

impl CheckResult {
    fn pass(id: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            passed: true,
            detail: detail.into(),
        }
    }

    fn fail(id: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            passed: false,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ReferenceRow {
    target_id: String,
    value: String,
}

pub fn run_checks(cond: &Condition, graph: &Graph, solution: &Solution) -> Vec<CheckResult> {
    let mut results = Vec::new();

    if cond.kx == Cap::Inf && cond.ko == Cap::Inf {
        match verify_reference_anchors(REFERENCE_CSV, cond, graph, solution) {
            Ok(anchor_results) => {
                let v1_ids = ["classic.n_states", "classic.n_states_canon", "classic.n_games"];
                results.push(aggregate("v1", &anchor_results, &v1_ids));
                results.push(aggregate("v2", &anchor_results, &["classic.value"]));
            }
            Err(error) => {
                results.push(CheckResult::fail("v1", error.clone()));
                results.push(CheckResult::fail("v2", error));
            }
        }
    }

    if let Some(result) = check_v3(cond, graph, solution) {
        results.push(result);
    }
    results.push(check_v4(graph, solution));
    results.push(check_v5(cond, graph, solution));
    results.push(check_v6(graph, solution));
    results.push(check_v7(cond, graph, solution));
    results.push(check_line_invariant(graph));
    results
}

pub fn verify_reference_anchors(csv: &str, cond: &Condition, graph: &Graph, solution: &Solution) -> Result<Vec<CheckResult>, String> {
    if cond.kx != Cap::Inf || cond.ko != Cap::Inf {
        return Ok(Vec::new());
    }
    validate_solution_shape(graph, solution).map_err(|result| result.detail)?;

    let expected = parse_reference(csv)?;
    let canonical_count = graph.states.iter().map(canonical).collect::<HashSet<_>>().len() as i64;
    let games = graph
        .n_games()
        .ok_or_else(|| "classic.n_games cannot be computed for a cyclic graph".to_owned())?;
    let actual = [
        ("classic.n_states", graph.states.len() as i64),
        ("classic.n_states_canon", canonical_count),
        ("classic.n_games", games as i64),
        ("classic.value", i64::from(solution.value[graph.root as usize])),
    ];

    actual
        .into_iter()
        .map(|(target_id, actual_value)| {
            let expected_value = expected.get(target_id).ok_or_else(|| format!("reference is missing {target_id}"))?;
            Ok(if actual_value == *expected_value {
                CheckResult::pass(target_id, format!("matched reference value {expected_value}"))
            } else {
                CheckResult::fail(target_id, format!("expected {expected_value}, got {actual_value}"))
            })
        })
        .collect()
}

fn parse_reference(csv: &str) -> Result<HashMap<String, i64>, String> {
    let mut values = HashMap::new();
    let mut reader = csv::Reader::from_reader(csv.as_bytes());
    for row in reader.deserialize::<ReferenceRow>() {
        let row = row.map_err(|error| format!("invalid reference row: {error}"))?;
        let value = row
            .value
            .parse::<i64>()
            .map_err(|error| format!("invalid value for reference {}: {} ({error})", row.target_id, row.value))?;
        if values.insert(row.target_id.clone(), value).is_some() {
            return Err(format!("duplicate reference target {}", row.target_id));
        }
    }
    Ok(values)
}

fn aggregate(id: &str, results: &[CheckResult], members: &[&str]) -> CheckResult {
    let selected: Vec<_> = members
        .iter()
        .filter_map(|member| results.iter().find(|result| result.id == **member))
        .collect();
    if selected.len() != members.len() {
        return CheckResult::fail(id, "one or more reference results are missing");
    }
    let failures: Vec<_> = selected
        .iter()
        .filter(|result| !result.passed)
        .map(|result| format!("{}: {}", result.id, result.detail))
        .collect();
    if failures.is_empty() {
        CheckResult::pass(id, format!("{} reference anchors matched", members.len()))
    } else {
        CheckResult::fail(id, failures.join("; "))
    }
}

pub fn check_v3(cond: &Condition, graph: &Graph, solution: &Solution) -> Option<CheckResult> {
    let mut obligations = Vec::new();
    match cond.rule {
        Rule::T1 | Rule::T3 => {
            for player in [Player::X, Player::O] {
                if cap_at_most(cond.cap(player), 2) {
                    obligations.push((!mover_has_winning_edge(graph, player), format!("{player} with cap <= 2 completed a line")));
                }
            }
            if cap_at_most(cond.kx, 2) && cap_at_most(cond.ko, 2) {
                obligations.push((
                    solution.value.iter().all(|value| *value == 0),
                    "not every state is a draw when both caps are <= 2".to_owned(),
                ));
            }
        }
        Rule::T2 => {
            for player in [Player::X, Player::O] {
                let cap = cond.cap(player);
                if cap_at_most(cap, 1) {
                    obligations.push((!mover_has_winning_edge(graph, player), format!("{player} with cap <= 1 completed a line")));
                } else if cap == Cap::Finite(2) {
                    obligations.push((
                        mover_has_winning_edge(graph, player),
                        format!("{player} with cap 2 never completed a T2 line"),
                    ));
                }
            }
            if cap_at_most(cond.kx, 1) && cap_at_most(cond.ko, 1) {
                obligations.push((
                    solution.value.iter().all(|value| *value == 0),
                    "not every state is a draw when both T2 caps are <= 1".to_owned(),
                ));
            }
        }
    }

    if obligations.is_empty() {
        return None;
    }
    let failures: Vec<_> = obligations
        .into_iter()
        .filter_map(|(passed, detail)| (!passed).then_some(detail))
        .collect();
    Some(if failures.is_empty() {
        CheckResult::pass("v3", "low-cap line and draw properties hold")
    } else {
        CheckResult::fail("v3", failures.join("; "))
    })
}

fn mover_has_winning_edge(graph: &Graph, player: Player) -> bool {
    graph
        .states
        .iter()
        .enumerate()
        .any(|(index, state)| state.to_move() == player && graph.successors(index as u32).iter().any(|edge| edge.outcome == Outcome::MoverWins))
}

const fn cap_at_most(cap: Cap, limit: u8) -> bool {
    matches!(cap, Cap::Finite(value) if value <= limit)
}

pub fn check_v4(graph: &Graph, solution: &Solution) -> CheckResult {
    let canonical_graph = Graph::enumerate_canonical(&graph.cond);
    let canonical_solution = retrograde(&canonical_graph);
    check_v4_against(graph, solution, &canonical_graph, &canonical_solution)
}

pub fn check_v4_against(graph: &Graph, solution: &Solution, canonical_graph: &Graph, canonical_solution: &Solution) -> CheckResult {
    if let Err(result) = validate_solution_shape(graph, solution) {
        return with_id(result, "v4");
    }
    if let Err(result) = validate_solution_shape(canonical_graph, canonical_solution) {
        return with_id(result, "v4");
    }
    if graph.cond != canonical_graph.cond {
        return CheckResult::fail("v4", "raw and canonical graphs use different conditions");
    }
    let raw_root = graph.root as usize;
    let canonical_root = canonical_graph.root as usize;
    if solution.value[raw_root] != canonical_solution.value[canonical_root] || solution.plies[raw_root] != canonical_solution.plies[canonical_root] {
        return CheckResult::fail(
            "v4",
            format!(
                "root differs between raw and canonical solvers: ({}, {:?}) versus ({}, {:?})",
                solution.value[raw_root],
                solution.plies[raw_root],
                canonical_solution.value[canonical_root],
                canonical_solution.plies[canonical_root]
            ),
        );
    }
    let mut weighted_states = 0_usize;
    let mut weighted_counts = [0_usize; 3];
    for (index, representative) in canonical_graph.states.iter().enumerate() {
        if canonical(representative) != *representative {
            return CheckResult::fail("v4", format!("canonical graph contains noncanonical state {index}"));
        }
        let orbit_size = 8 / stabilizer(representative).len();
        weighted_states += orbit_size;
        weighted_counts[value_index(canonical_solution.value[index])] += orbit_size;
    }

    let mut direct_counts = [0_usize; 3];
    for (index, (state, value)) in graph.states.iter().zip(&solution.value).enumerate() {
        direct_counts[value_index(*value)] += 1;
        let representative = canonical(state);
        let Some(canonical_id) = canonical_graph.state_id(&representative) else {
            return CheckResult::fail("v4", format!("raw state {index} has no canonical representative"));
        };
        let canonical_index = canonical_id as usize;
        if canonical_solution.value[canonical_index] != *value || canonical_solution.plies[canonical_index] != solution.plies[index] {
            return CheckResult::fail("v4", format!("raw state {index} differs from its canonical solution entry"));
        }
    }
    if weighted_states != graph.states.len() || weighted_counts != direct_counts {
        return CheckResult::fail(
            "v4",
            format!(
                "orbit-weighted counts differ: states {weighted_states}/{}, values {weighted_counts:?}/{direct_counts:?}",
                graph.states.len()
            ),
        );
    }
    CheckResult::pass(
        "v4",
        format!(
            "{} canonical states reproduce {} raw states and value counts",
            canonical_graph.states.len(),
            graph.states.len()
        ),
    )
}

fn value_index(value: i8) -> usize {
    match value {
        -1 => 0,
        0 => 1,
        1 => 2,
        _ => unreachable!("solution values must be -1, 0, or 1"),
    }
}

pub fn check_v5(cond: &Condition, graph: &Graph, solution: &Solution) -> CheckResult {
    if let Err(result) = validate_solution_shape(graph, solution) {
        return with_id(result, "v5");
    }
    let iterated = value_iteration(&graph.states, cond, None);
    if iterated.value != solution.value {
        let index = iterated
            .value
            .iter()
            .zip(&solution.value)
            .position(|(left, right)| left != right)
            .unwrap_or(0);
        return CheckResult::fail(
            "v5",
            format!(
                "value iteration differs at state {index}: {} versus {}",
                iterated.value[index], solution.value[index]
            ),
        );
    }
    if iterated.plies != solution.plies {
        let index = iterated
            .plies
            .iter()
            .zip(&solution.plies)
            .position(|(left, right)| left != right)
            .unwrap_or(0);
        return CheckResult::fail(
            "v5",
            format!(
                "first decisive iteration differs at state {index}: {:?} versus {:?}",
                iterated.plies[index], solution.plies[index]
            ),
        );
    }
    CheckResult::pass("v5", "independent value iteration matches every value and distance")
}

pub fn check_v6(graph: &Graph, solution: &Solution) -> CheckResult {
    if let Err(result) = validate_solution_shape(graph, solution) {
        return with_id(result, "v6");
    }
    for state in 0..graph.states.len() {
        let value = solution.value[state];
        let plies = solution.plies[state];
        let successors = graph.successors(state as u32);
        match (value, plies) {
            (-1, Some(0)) if graph.terminal[state] == Terminal::MoverWon => {}
            (1, Some(n)) if n > 0 => {
                let mut losing_distances = Vec::new();
                for edge in successors {
                    if edge.outcome == Outcome::MoverWins {
                        losing_distances.push(0);
                    } else if solution.value[edge.to as usize] == -1 {
                        if let Some(distance) = solution.plies[edge.to as usize] {
                            losing_distances.push(distance);
                        }
                    }
                }
                if losing_distances.iter().min().copied() != Some(n - 1) {
                    return CheckResult::fail(
                        "v6",
                        format!("state {state} wins in {n}, but losing successor distances are {losing_distances:?}"),
                    );
                }
            }
            (-1, Some(n)) if n > 0 => {
                let mut winning_distances = Vec::new();
                for edge in successors {
                    if edge.outcome != Outcome::Continue || solution.value[edge.to as usize] != 1 || solution.plies[edge.to as usize].is_none() {
                        return CheckResult::fail("v6", format!("state {state} has a successor that is not a solved win"));
                    }
                    winning_distances.push(solution.plies[edge.to as usize].unwrap());
                }
                if winning_distances.iter().max().copied() != Some(n - 1) {
                    return CheckResult::fail(
                        "v6",
                        format!("state {state} loses in {n}, but winning successor distances are {winning_distances:?}"),
                    );
                }
            }
            (0, None) => {
                let has_losing = successors
                    .iter()
                    .any(|edge| edge.outcome == Outcome::MoverWins || solution.value[edge.to as usize] == -1);
                let has_draw = successors.iter().any(|edge| solution.value[edge.to as usize] == 0);
                if has_losing || (!successors.is_empty() && !has_draw) {
                    return CheckResult::fail("v6", format!("state {state} does not preserve a draw"));
                }
            }
            _ => {
                return CheckResult::fail("v6", format!("state {state} has invalid value/distance ({value}, {plies:?})"));
            }
        }
    }
    CheckResult::pass("v6", "all local minimax distance properties hold")
}

pub fn check_v7(cond: &Condition, graph: &Graph, solution: &Solution) -> CheckResult {
    if let Err(result) = validate_solution_shape(graph, solution) {
        return with_id(result, "v7");
    }
    let max_plies = graph.states.len() as u64 + 1;
    let game = match play_game(
        *cond,
        Box::new(OptimalPolicy::from_graph(graph, solution)),
        Box::new(OptimalPolicy::from_graph(graph, solution)),
        Box::new(NullRecorder),
        0,
        PlayOptions::new("check.v7", max_plies, true),
    ) {
        Ok(game) => game,
        Err(error) => return CheckResult::fail("v7", format!("optimal game failed: {error}")),
    };
    let root = graph.root as usize;
    let root_value = solution.value[root];
    let expected_result = match (root_value, cond.first) {
        (1, Player::X) | (-1, Player::O) => GameResult::XWin,
        (1, Player::O) | (-1, Player::X) => GameResult::OWin,
        (0, _) => GameResult::Draw,
        _ => {
            return CheckResult::fail("v7", format!("root has invalid value {root_value}"));
        }
    };
    if game.result != expected_result {
        return CheckResult::fail("v7", format!("optimal game result is {:?}, expected {expected_result:?}", game.result));
    }
    if let Some(expected_plies) = solution.plies[root] {
        if game.plies != u64::from(expected_plies) || game.reason != EndReason::Line {
            return CheckResult::fail(
                "v7",
                format!(
                    "optimal game ended in {} plies by {:?}, expected {expected_plies} plies by line",
                    game.plies, game.reason
                ),
            );
        }
    } else if game.reason == EndReason::Cap {
        return CheckResult::fail("v7", "optimal draw reached the safety cap instead of a repeat or terminal draw");
    }
    CheckResult::pass("v7", format!("optimal socsim game ended in {} plies by {:?}", game.plies, game.reason))
}

pub fn check_line_invariant(graph: &Graph) -> CheckResult {
    for (index, state) in graph.states.iter().enumerate() {
        if graph.terminal[index] == Terminal::MoverWon && !graph.predecessors(index as u32).iter().any(|edge| edge.outcome == Outcome::MoverWins) {
            return CheckResult::fail("r41", format!("state {index} is a winning terminal without a winning predecessor"));
        }
        if graph.terminal[index] != Terminal::MoverWon && (has_line(state.cells_of(Player::X)) || has_line(state.cells_of(Player::O))) {
            return CheckResult::fail("r41", format!("state {index} is nonterminal but contains a line"));
        }
        for edge in graph.successors(index as u32) {
            if edge.outcome != Outcome::MoverWins {
                continue;
            }
            let mover = state.to_move();
            let next = graph.states[edge.to as usize];
            let mover_has_line = if graph.cond.rule == Rule::T2 {
                let mut judging_cells = state.cells_in_order(mover);
                judging_cells.push(edge.cell);
                has_line(mask(&judging_cells))
            } else {
                has_line(next.cells_of(mover))
            };
            if !mover_has_line || has_line(next.cells_of(mover.other())) {
                return CheckResult::fail("r41", format!("winning edge from state {index} does not have exactly the mover's line"));
            }
        }
    }
    CheckResult::pass("r41", "nonterminal states have no line and every winning edge has only the mover's line")
}

fn mask(cells: &[u8]) -> u16 {
    cells.iter().fold(0_u16, |mask, cell| mask | (1 << cell))
}

fn validate_solution_shape(graph: &Graph, solution: &Solution) -> Result<(), CheckResult> {
    if solution.value.len() != graph.states.len() || solution.plies.len() != graph.states.len() {
        return Err(CheckResult::fail(
            "shape",
            format!(
                "solution has {} values and {} distances for {} states",
                solution.value.len(),
                solution.plies.len(),
                graph.states.len()
            ),
        ));
    }
    if let Some((index, value)) = solution.value.iter().enumerate().find(|(_, value)| !matches!(value, -1..=1)) {
        return Err(CheckResult::fail("shape", format!("state {index} has invalid value {value}")));
    }
    Ok(())
}

fn with_id(mut result: CheckResult, id: &str) -> CheckResult {
    result.id = id.to_owned();
    result
}
