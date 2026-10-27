use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;
use std::rc::Rc;

use clap::{Args, Parser, Subcommand, ValueEnum};
use fifo_ttt_simulation::analysis::{best_moves, MoveClass};
use fifo_ttt_simulation::checks::{run_checks, CheckResult};
use fifo_ttt_simulation::config::{Cap, Condition, Player, Rule, K_VALUES};
use fifo_ttt_simulation::graph::Graph;
use fifo_ttt_simulation::policy::OptimalPolicy;
use fifo_ttt_simulation::record::{metric_name, record_solve, start_run, write_run_file, SharedRecorder};
use fifo_ttt_simulation::retro::{retrograde, Solution};
use fifo_ttt_simulation::simulation::{play_game, PlayOptions};
use runvault_socsim::RunHandle;
use serde_json::json;
use socsim_core::NullRecorder;

#[derive(Parser, Debug)]
#[command(name = "fifo-ttt", about = "Solve finite-memory tic-tac-toe")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Solve all 25 cap pairs for one rule and record the run.
    Solve {
        #[arg(long, value_enum)]
        rule: RuleArg,
        #[command(flatten)]
        mode: RunMode,
    },
    /// Show one solved condition without creating a run.
    Show {
        #[arg(long, value_enum)]
        rule: RuleArg,
        #[arg(long)]
        kx: String,
        #[arg(long)]
        ko: String,
        #[arg(long, value_enum, default_value_t = PlayerArg::X)]
        first: PlayerArg,
        #[command(flatten)]
        mode: RunMode,
    },
}

#[derive(Args, Clone, Copy, Debug)]
struct RunMode {
    /// Create a scratch run excluded from production analyses.
    #[arg(long, required_unless_present = "prod", conflicts_with = "prod")]
    scratch: bool,
    /// Create a production run.
    #[arg(long, required_unless_present = "scratch", conflicts_with = "scratch")]
    prod: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum RuleArg {
    T1,
    T2,
    T3,
}

impl From<RuleArg> for Rule {
    fn from(value: RuleArg) -> Self {
        match value {
            RuleArg::T1 => Self::T1,
            RuleArg::T2 => Self::T2,
            RuleArg::T3 => Self::T3,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum PlayerArg {
    #[default]
    X,
    O,
}

impl From<PlayerArg> for Player {
    fn from(value: PlayerArg) -> Self {
        match value {
            PlayerArg::X => Self::X,
            PlayerArg::O => Self::O,
        }
    }
}

struct ConditionResult {
    cond: Condition,
    graph: Graph,
    solution: Solution,
    classes: Vec<MoveClass>,
    canonical_states: usize,
    checks: Vec<CheckResult>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Command::Solve { rule, mode } => solve_command(rule.into(), mode.scratch),
        Command::Show {
            rule,
            kx,
            ko,
            first,
            mode: _,
        } => show_command(rule.into(), parse_cap(&kx)?, parse_cap(&ko)?, first.into()),
    }
}

fn solve_command(rule: Rule, scratch: bool) -> Result<(), Box<dyn Error>> {
    let parameters = json!({
        "rule": rule.to_string(),
        "first": "x",
        "horizon": null,
        "k_values": [1, 2, 3, 4, "inf"],
    });
    let label = format!("{}, first=X, H=none", rule.to_string().to_uppercase());
    let (recorder, handle) = start_run("solve", &parameters, &label, scratch)?;
    let shared = Rc::new(RefCell::new(recorder));

    match execute_solve(rule, &handle, &shared) {
        Ok(()) => {
            let dir = handle.finish()?;
            println!("Run directory: {}", dir.display());
            Ok(())
        }
        Err(error) => {
            if let Some(dir) = handle.dir() {
                eprintln!("Failed run directory: {}", dir.display());
            }
            let message = error.to_string();
            let _ = handle.fail("check", message.clone());
            Err(message.into())
        }
    }
}

fn execute_solve(rule: Rule, handle: &RunHandle, recorder: &Rc<RefCell<runvault_socsim::RunvaultRecorder>>) -> Result<(), Box<dyn Error>> {
    let mut metrics = Vec::new();
    let mut all_classes = Vec::new();
    let mut checks_by_id: BTreeMap<String, bool> = BTreeMap::new();
    let mut check_log = String::new();
    let mut done = 0;

    for kx in K_VALUES {
        for ko in K_VALUES {
            let result = solve_condition(Condition::new(rule, kx, ko, Player::X));
            append_metrics(&result, &mut metrics);
            append_best_moves(&result, &mut all_classes);
            for check in &result.checks {
                checks_by_id
                    .entry(check.id.clone())
                    .and_modify(|passed| *passed &= check.passed)
                    .or_insert(check.passed);
                check_log.push_str(&format!(
                    "kx={}, ko={}, {}: {} - {}\n",
                    kx.label(),
                    ko.label(),
                    check.id,
                    if check.passed { "pass" } else { "fail" },
                    check.detail
                ));
            }

            play_game(
                result.cond,
                Box::new(OptimalPolicy::from_graph(&result.graph, &result.solution)),
                Box::new(OptimalPolicy::from_graph(&result.graph, &result.solution)),
                Box::new(SharedRecorder::new(Rc::clone(recorder))),
                0,
                PlayOptions::new(
                    format!("pv.kx{}_ko{}", kx.label(), ko.label()),
                    result.graph.states.len() as u64 + 1,
                    true,
                ),
            )?;

            done += 1;
            eprintln!("Solved {done}/25 conditions");
        }
    }

    for required in ["v1", "v2", "v3", "v4", "v5", "v6", "v7", "r41"] {
        checks_by_id.entry(required.to_owned()).or_insert(false);
    }
    let failures = checks_by_id.values().filter(|passed| !**passed).count();
    for (id, passed) in &checks_by_id {
        metrics.push((format!("check.{id}"), f64::from(*passed)));
    }
    metrics.push(("checks_failed".to_owned(), failures as f64));

    record_solve(handle, &metrics)?;
    write_run_file(handle, "artifacts/best_moves.csv", &best_moves_csv(&all_classes)?)?;
    write_run_file(handle, "logs/checks.log", &check_log)?;
    if failures > 0 {
        return Err(format!("{failures} validation checks failed").into());
    }
    println!("Checks passed: {}/{}", checks_by_id.len(), checks_by_id.len());
    Ok(())
}

fn solve_condition(cond: Condition) -> ConditionResult {
    let graph = Graph::enumerate(&cond);
    let solution = retrograde(&graph);
    let canonical_graph = Graph::enumerate_canonical(&cond);
    let canonical_solution = retrograde(&canonical_graph);
    debug_assert_eq!(
        solution.value[graph.root as usize],
        canonical_solution.value[canonical_graph.root as usize]
    );
    debug_assert_eq!(
        solution.plies[graph.root as usize],
        canonical_solution.plies[canonical_graph.root as usize]
    );
    let classes = best_moves(&graph, &solution, graph.root);
    let checks = run_checks(&cond, &graph, &solution);
    ConditionResult {
        cond,
        canonical_states: canonical_graph.states.len(),
        graph,
        solution,
        classes,
        checks,
    }
}

fn append_metrics(result: &ConditionResult, metrics: &mut Vec<(String, f64)>) {
    let root = result.graph.root as usize;
    let root_value_x = value_from_x(result.cond, result.solution.value[root]);
    metrics.push((metric_name("value", result.cond.kx, result.cond.ko), f64::from(root_value_x)));
    if let Some(plies) = result.solution.plies[root] {
        metrics.push((metric_name("plies", result.cond.kx, result.cond.ko), f64::from(plies)));
    }
    metrics.push((metric_name("n_states", result.cond.kx, result.cond.ko), result.graph.states.len() as f64));
    metrics.push((
        metric_name("n_states_canon", result.cond.kx, result.cond.ko),
        result.canonical_states as f64,
    ));
    for (name, value) in [("n_win", 1), ("n_draw", 0), ("n_lose", -1)] {
        metrics.push((
            metric_name(name, result.cond.kx, result.cond.ko),
            result.solution.value.iter().filter(|candidate| **candidate == value).count() as f64,
        ));
    }
    if let Some(games) = result.graph.n_games() {
        metrics.push((metric_name("n_games", result.cond.kx, result.cond.ko), games as f64));
    }
    metrics.push((
        metric_name("n_best_move_classes", result.cond.kx, result.cond.ko),
        result.classes.len() as f64,
    ));
}

fn value_from_x(cond: Condition, value_to_move: i8) -> i8 {
    if cond.first == Player::X {
        value_to_move
    } else {
        -value_to_move
    }
}

fn append_best_moves(result: &ConditionResult, rows: &mut Vec<(Cap, Cap, MoveClass)>) {
    rows.extend(result.classes.iter().cloned().map(|class| (result.cond.kx, result.cond.ko, class)));
}

fn best_moves_csv(rows: &[(Cap, Cap, MoveClass)]) -> Result<String, Box<dyn Error>> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(["kx", "ko", "class_id", "cells", "value_after_x", "plies_after"])?;
    for (kx, ko, class) in rows {
        let cells = class.cells.iter().map(u8::to_string).collect::<Vec<_>>().join("|");
        writer.write_record([
            kx.label().to_owned(),
            ko.label().to_owned(),
            class.class_id.to_string(),
            cells,
            class.value_after_x.to_string(),
            class.plies_after.map(|value| value.to_string()).unwrap_or_default(),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}

fn show_command(rule: Rule, kx: Cap, ko: Cap, first: Player) -> Result<(), Box<dyn Error>> {
    let result = solve_condition(Condition::new(rule, kx, ko, first));
    let root = result.graph.root as usize;
    let game = play_game(
        result.cond,
        Box::new(OptimalPolicy::from_graph(&result.graph, &result.solution)),
        Box::new(OptimalPolicy::from_graph(&result.graph, &result.solution)),
        Box::new(NullRecorder),
        0,
        PlayOptions::new(
            format!("show.kx{}_ko{}", kx.label(), ko.label()),
            result.graph.states.len() as u64 + 1,
            true,
        ),
    )?;
    println!("rule: {rule}");
    println!("kx: {}", kx.label());
    println!("ko: {}", ko.label());
    println!("first: {first}");
    println!("value_x: {}", value_from_x(result.cond, result.solution.value[root]));
    match result.solution.plies[root] {
        Some(plies) => println!("plies: {plies}"),
        None => println!("plies: draw"),
    }
    for class in result.classes {
        println!(
            "best_move_class {}: {:?} (value_after_x={}, plies_after={:?})",
            class.class_id, class.cells, class.value_after_x, class.plies_after
        );
    }
    println!(
        "pv: result={}, plies={}, reason={}, repeat_to={:?}",
        game.result.as_str(),
        game.plies,
        game.reason.as_str(),
        game.repeat_to
    );
    Ok(())
}

fn parse_cap(raw: &str) -> Result<Cap, Box<dyn Error>> {
    match raw.to_ascii_lowercase().as_str() {
        "1" => Ok(Cap::Finite(1)),
        "2" => Ok(Cap::Finite(2)),
        "3" => Ok(Cap::Finite(3)),
        "4" => Ok(Cap::Finite(4)),
        "inf" | "infinity" => Ok(Cap::Inf),
        _ => Err(format!("cap must be one of 1, 2, 3, 4, inf; got {raw}").into()),
    }
}
