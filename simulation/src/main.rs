use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Instant;

use clap::{Args, Parser, Subcommand, ValueEnum};
use fifo_ttt_simulation::analysis::{best_moves, MoveClass};
use fifo_ttt_simulation::checks::{check_v10, check_v11, check_v8, check_v9, run_checks, CheckResult};
use fifo_ttt_simulation::config::{sweep_specs, Cap, Condition, Player, Rule, SolveSpec, K_VALUES};
use fifo_ttt_simulation::graph::Graph;
use fifo_ttt_simulation::iterate::value_iteration;
use fifo_ttt_simulation::naive::naive_negamax_with_progress;
use fifo_ttt_simulation::policy::{OptimalPolicy, Policy, UniformRandomPolicy};
use fifo_ttt_simulation::record::{metric_name, record_solve, start_play_run, start_run, start_sweep_child, write_run_file, SharedRecorder};
use fifo_ttt_simulation::retro::{retrograde, Solution};
use fifo_ttt_simulation::simulation::{play_game, PlayOptions};
use fifo_ttt_simulation::world::GameResult;
use runvault::{Run, RunOptions};
use runvault_socsim::{RunHandle, RunvaultRecorder};
use serde_json::json;
use socsim_core::{derive_seed, NullRecorder};

const DEFAULT_NODE_CAP: u64 = 1_000_000_000;
const DEFAULT_GAMES: u64 = 100;
const DEFAULT_SEED: u64 = 20_261_027;

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
        /// Player that moves first.
        #[arg(long, value_enum, default_value_t = PlayerArg::X)]
        first: PlayerArg,
        /// Treat games unresolved after this many plies as draws.
        #[arg(long)]
        horizon: Option<u32>,
        #[command(flatten)]
        mode: RunMode,
    },
    /// Run the 15-point rule, horizon, and first-player sweep.
    All {
        #[command(flatten)]
        mode: RunMode,
    },
    /// Run explicit-stack naive negamax for all 25 cap pairs.
    Naive {
        #[arg(long, value_enum)]
        rule: RuleArg,
        /// Maximum generated search-tree nodes per cap pair.
        #[arg(long, default_value_t = DEFAULT_NODE_CAP)]
        node_cap: u64,
        #[command(flatten)]
        mode: RunMode,
    },
    /// Play optimal-versus-random games in both seat assignments.
    Play {
        #[arg(long, value_enum, default_value_t = RuleArg::T3)]
        rule: RuleArg,
        #[arg(long, value_enum, default_value_t = PolicyArg::Optimal)]
        x: PolicyArg,
        #[arg(long, value_enum, default_value_t = PolicyArg::Random)]
        o: PolicyArg,
        /// Games per cap pair and seat assignment.
        #[arg(long, default_value_t = DEFAULT_GAMES, value_parser = clap::value_parser!(u64).range(1..))]
        games: u64,
        /// Master seed used to derive one seed per game.
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
        /// Ply cutoff; by default each condition uses its reachable-state count plus one.
        #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
        max_plies: Option<u64>,
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

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum RuleArg {
    T1,
    T2,
    #[default]
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum PolicyArg {
    Optimal,
    Random,
}

impl PolicyArg {
    const fn label(self) -> &'static str {
        match self {
            Self::Optimal => "optimal",
            Self::Random => "random",
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

enum NaiveMessage {
    Progress {
        index: usize,
        nodes: u64,
    },
    Finished {
        index: usize,
        result: fifo_ttt_simulation::naive::NaiveResult,
        check: CheckResult,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        Command::Solve { rule, first, horizon, mode } => solve_command(
            SolveSpec {
                rule: rule.into(),
                first: first.into(),
                horizon,
            },
            mode.scratch,
        ),
        Command::All { mode } => all_command(mode.scratch),
        Command::Naive { rule, node_cap, mode } => naive_command(rule.into(), node_cap, mode.scratch),
        Command::Play {
            rule,
            x,
            o,
            games,
            seed,
            max_plies,
            mode,
        } => play_command(rule.into(), x, o, games, seed, max_plies, mode.scratch),
        Command::Show {
            rule,
            kx,
            ko,
            first,
            mode: _,
        } => show_command(rule.into(), parse_cap(&kx)?, parse_cap(&ko)?, first.into()),
    }
}

fn solve_parameters(spec: SolveSpec) -> serde_json::Value {
    json!({
        "rule": spec.rule.to_string(),
        "first": spec.first.to_string(),
        "horizon": spec.horizon,
        "k_values": [1, 2, 3, 4, "inf"],
    })
}

fn solve_label(spec: SolveSpec) -> String {
    format!(
        "{}, first={}, H={}",
        spec.rule.to_string().to_uppercase(),
        spec.first.to_string().to_uppercase(),
        spec.horizon.map_or_else(|| "none".to_owned(), |value| value.to_string())
    )
}

fn solve_command(spec: SolveSpec, scratch: bool) -> Result<(), Box<dyn Error>> {
    let parameters = solve_parameters(spec);
    let (recorder, handle) = start_run("solve", &parameters, &solve_label(spec), scratch)?;
    finish_solve_run(spec, recorder, handle)
}

fn finish_solve_run(spec: SolveSpec, recorder: RunvaultRecorder, handle: RunHandle) -> Result<(), Box<dyn Error>> {
    let shared = Rc::new(RefCell::new(recorder));
    match execute_solve(spec, &handle, &shared) {
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

fn execute_solve(spec: SolveSpec, handle: &RunHandle, recorder: &Rc<RefCell<RunvaultRecorder>>) -> Result<(), Box<dyn Error>> {
    let mut metrics = Vec::new();
    let mut all_classes = Vec::new();
    let mut checks_by_id: BTreeMap<String, bool> = BTreeMap::new();
    let mut check_log = String::new();
    let mut done = 0;

    for kx in K_VALUES {
        for ko in K_VALUES {
            let cond = Condition::new(spec.rule, kx, ko, spec.first).with_horizon(spec.horizon);
            let result = solve_condition(cond);
            append_metrics(&result, &mut metrics);
            append_best_moves(&result, &mut all_classes);
            merge_checks(&result.checks, &mut checks_by_id, &mut check_log, kx, ko);

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
            log_progress(handle.dir().as_deref(), &format!("Solved conditions {done}/25"))?;
        }
    }

    for required in ["v1", "v2", "v3", "v4", "v5", "v6", "v7", "r41"] {
        checks_by_id.entry(required.to_owned()).or_insert(false);
    }
    if spec.horizon.is_some() {
        checks_by_id.entry("v8".to_owned()).or_insert(false);
    }
    if spec.first == Player::O {
        checks_by_id.entry("v9".to_owned()).or_insert(false);
    }
    let failures = append_check_metrics(&checks_by_id, &mut metrics);

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
    let unbounded = retrograde(&graph);
    let canonical_graph = Graph::enumerate_canonical(&cond);
    let canonical_solution = retrograde(&canonical_graph);
    debug_assert_eq!(
        unbounded.value[graph.root as usize],
        canonical_solution.value[canonical_graph.root as usize]
    );
    debug_assert_eq!(
        unbounded.plies[graph.root as usize],
        canonical_solution.plies[canonical_graph.root as usize]
    );

    let mut checks = run_checks(&cond, &graph, &unbounded);
    let solution = if let Some(horizon) = cond.horizon {
        let bounded = value_iteration(&graph.states, &cond, Some(horizon));
        checks.push(check_v8(&graph, &unbounded, &bounded, horizon));
        bounded
    } else {
        unbounded
    };
    if cond.first == Player::O {
        checks.push(check_v9(cond.rule, cond.kx, cond.ko));
    }
    let classes = best_moves(&graph, &solution, graph.root);
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
    if result.cond.horizon.is_some() {
        return;
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

fn merge_checks(checks: &[CheckResult], checks_by_id: &mut BTreeMap<String, bool>, check_log: &mut String, kx: Cap, ko: Cap) {
    for check in checks {
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
}

fn append_check_metrics(checks: &BTreeMap<String, bool>, metrics: &mut Vec<(String, f64)>) -> usize {
    let failures = checks.values().filter(|passed| !**passed).count();
    for (id, passed) in checks {
        metrics.push((format!("check.{id}"), f64::from(*passed)));
    }
    metrics.push(("checks_failed".to_owned(), failures as f64));
    failures
}

fn all_command(scratch: bool) -> Result<(), Box<dyn Error>> {
    let specs = sweep_specs();
    let parameters = json!({
        "rules": ["t1", "t2", "t3"],
        "horizons": [null, 20, 50, 100],
        "first_players": ["x", "o"],
        "children": specs.len(),
    });
    let options = RunOptions::new("fifo-ttt", "all")
        .repo_id("fifo-tic-tac-toe")
        .domain("analysis")
        .scratch(scratch)
        .parameters(&parameters)?
        .label("15-point rule and sensitivity sweep")
        .sweep_parent();
    let parent = Run::start(options)?;
    let sweep_id = parent.sweep_id().ok_or("sweep parent has no sweep id")?.to_owned();
    let parent_uid = parent.run_uid().to_owned();
    let parent_dir = parent.dir().to_path_buf();
    let started = Instant::now();

    for (index, spec) in specs.iter().copied().enumerate() {
        let parameters = solve_parameters(spec);
        let (recorder, handle) = start_sweep_child(&parameters, &solve_label(spec), scratch, &sweep_id, &parent_uid, index, specs.len())?;
        if let Err(error) = finish_solve_run(spec, recorder, handle) {
            let message = format!("sweep child {index} failed: {error}");
            let failed_dir = parent.fail("child", message.clone())?;
            eprintln!("Failed sweep directory: {}", failed_dir.display());
            return Err(message.into());
        }
        log_progress(Some(&parent_dir), &format!("Completed child runs {}/15", index + 1))?;
    }

    let dir = parent.finish()?;
    println!("Sweep directory: {}", dir.display());
    println!("Sweep duration: {:.3}s", started.elapsed().as_secs_f64());
    Ok(())
}

fn naive_command(rule: Rule, node_cap: u64, scratch: bool) -> Result<(), Box<dyn Error>> {
    let parameters = json!({
        "rule": rule.to_string(),
        "first": "x",
        "horizon": null,
        "k_values": [1, 2, 3, 4, "inf"],
        "node_cap": node_cap,
    });
    let label = format!("{} naive, cap={node_cap}", rule.to_string().to_uppercase());
    let (recorder, handle) = start_run("naive", &parameters, &label, scratch)?;
    let _recorder = recorder;
    let run_dir = handle.dir().ok_or("run is already closed")?;
    let started = Instant::now();
    let mut metrics = Vec::new();
    let mut check_log = String::new();
    let mut judged = 0_u64;
    let mut unjudged = 0_u64;
    let mut all_match = true;
    let conditions: Vec<_> = K_VALUES
        .into_iter()
        .flat_map(|kx| K_VALUES.into_iter().map(move |ko| Condition::new(rule, kx, ko, Player::X)))
        .collect();
    let mut outputs = vec![None; conditions.len()];
    let (sender, receiver) = mpsc::channel();
    let next_condition = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(1, |count| count.get().min(8));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let sender = sender.clone();
            let next_condition = &next_condition;
            let conditions = &conditions;
            scope.spawn(move || loop {
                let index = next_condition.fetch_add(1, Ordering::Relaxed);
                let Some(cond) = conditions.get(index).copied() else {
                    break;
                };
                let expected_value = {
                    let graph = Graph::enumerate(&cond);
                    let solution = retrograde(&graph);
                    solution.value[graph.root as usize]
                };
                let result = naive_negamax_with_progress(&cond, node_cap, |nodes| {
                    let _ = sender.send(NaiveMessage::Progress { index, nodes });
                });
                let check = check_v10(&cond, result, expected_value);
                let _ = sender.send(NaiveMessage::Finished { index, result, check });
            });
        }
        drop(sender);
        let mut done = 0;
        for message in receiver {
            match message {
                NaiveMessage::Progress { index, nodes } => {
                    let cond = conditions[index];
                    let condition_label = format!("kx{}_ko{}", cond.kx.label(), cond.ko.label());
                    log_progress(Some(&run_dir), &format!("Naive {condition_label}: conditions {done}/25, nodes {nodes}"))?;
                }
                NaiveMessage::Finished { index, result, check } => {
                    outputs[index] = Some((result, check));
                    done += 1;
                    let cond = conditions[index];
                    let condition_label = format!("kx{}_ko{}", cond.kx.label(), cond.ko.label());
                    log_progress(
                        Some(&run_dir),
                        &format!("Naive conditions {done}/25: {condition_label}, nodes {}", result.nodes),
                    )?;
                }
            }
        }
        Ok::<(), Box<dyn Error>>(())
    })?;

    for (cond, output) in conditions.into_iter().zip(outputs) {
        let (result, check) = output.ok_or("naive worker did not return a result")?;
        all_match &= check.passed;
        check_log.push_str(&format!(
            "kx={}, ko={}, v10: {} - {}\n",
            cond.kx.label(),
            cond.ko.label(),
            if check.passed { "pass" } else { "fail" },
            check.detail
        ));
        metrics.push((metric_name("naive_capped", cond.kx, cond.ko), f64::from(result.capped)));
        if result.capped {
            unjudged += 1;
        } else {
            judged += 1;
            metrics.push((metric_name("naive_nodes", cond.kx, cond.ko), result.nodes as f64));
        }
    }

    metrics.push(("naive_judged".to_owned(), judged as f64));
    metrics.push(("naive_unjudged".to_owned(), unjudged as f64));
    metrics.push(("check.v10".to_owned(), f64::from(all_match)));
    metrics.push(("checks_failed".to_owned(), f64::from(!all_match)));
    record_solve(&handle, &metrics)?;
    write_run_file(&handle, "logs/checks.log", &check_log)?;
    if !all_match {
        return fail_handle(handle, "check", "V-10 failed".to_owned());
    }
    let dir = handle.finish()?;
    println!("Run directory: {}", dir.display());
    println!("Checks passed: 1/1");
    println!("Naive judged: {judged}, capped: {unjudged}");
    println!("Naive duration: {:.3}s", started.elapsed().as_secs_f64());
    Ok(())
}

fn play_command(rule: Rule, x: PolicyArg, o: PolicyArg, games: u64, seed: u64, max_plies: Option<u64>, scratch: bool) -> Result<(), Box<dyn Error>> {
    if x == o {
        return Err("play requires exactly one optimal policy and one random policy".into());
    }
    let parameters = json!({
        "rule": rule.to_string(),
        "first": "x",
        "horizon": null,
        "k_values": [1, 2, 3, 4, "inf"],
        "n_games": games,
        "policies": {"x": x.label(), "o": o.label(), "swap": true},
        "seed": seed,
        "max_plies": max_plies,
    });
    let label = format!("{} optimal versus random", rule.to_string().to_uppercase());
    let (recorder, handle) = start_play_run(&parameters, &label, scratch, seed)?;
    let shared = Rc::new(RefCell::new(recorder));
    let mut metrics = Vec::new();
    let mut check_log = String::new();
    let mut all_passed = true;
    let mut condition_index = 0_u64;

    for kx in K_VALUES {
        for ko in K_VALUES {
            let cond = Condition::new(rule, kx, ko, Player::X);
            let graph = Graph::enumerate(&cond);
            let solution = retrograde(&graph);
            let root = graph.root as usize;
            let cutoff = max_plies.unwrap_or(graph.states.len() as u64 + 1);
            let optimal_policy = OptimalPolicy::from_graph(&graph, &solution);
            let seatings = [(x, o), (o, x)];
            let mut counts = [0_u64; 3];

            for (seating, (x_policy, o_policy)) in seatings.into_iter().enumerate() {
                let optimal = if x_policy == PolicyArg::Optimal { Player::X } else { Player::O };
                let expected_value = if optimal == Player::X {
                    solution.value[root]
                } else {
                    -solution.value[root]
                };
                for game_number in 0..games {
                    let within_condition = seating as u64 * games + game_number;
                    let game_seed = derive_seed(seed, &[condition_index, within_condition]);
                    let game = play_game(
                        cond,
                        make_policy(x_policy, &optimal_policy),
                        make_policy(o_policy, &optimal_policy),
                        Box::new(SharedRecorder::new(Rc::clone(&shared))),
                        game_seed,
                        PlayOptions::new(
                            format!("play.kx{}_ko{}.seat{seating}.game{game_number}", kx.label(), ko.label()),
                            cutoff,
                            false,
                        ),
                    )?;
                    let check = check_v11(expected_value, solution.plies[root], optimal, &game);
                    all_passed &= check.passed;
                    if !check.passed {
                        check_log.push_str(&format!(
                            "kx={}, ko={}, seating={seating}, game={game_number}: fail - {}\n",
                            kx.label(),
                            ko.label(),
                            check.detail
                        ));
                    }
                    let optimal_result = match (optimal, game.result) {
                        (_, GameResult::Draw) => 1,
                        (Player::X, GameResult::XWin) | (Player::O, GameResult::OWin) => 2,
                        _ => 0,
                    };
                    counts[optimal_result] += 1;
                }
            }

            metrics.push((metric_name("play_lose", kx, ko), counts[0] as f64));
            metrics.push((metric_name("play_draw", kx, ko), counts[1] as f64));
            metrics.push((metric_name("play_win", kx, ko), counts[2] as f64));
            condition_index += 1;
            log_progress(handle.dir().as_deref(), &format!("Played conditions {condition_index}/25"))?;
        }
    }

    metrics.push(("check.v11".to_owned(), f64::from(all_passed)));
    metrics.push(("checks_failed".to_owned(), f64::from(!all_passed)));
    record_solve(&handle, &metrics)?;
    write_run_file(&handle, "logs/checks.log", &check_log)?;
    if !all_passed {
        return fail_handle(handle, "check", "V-11 failed".to_owned());
    }
    let dir = handle.finish()?;
    println!("Run directory: {}", dir.display());
    println!("Checks passed: 1/1");
    Ok(())
}

fn make_policy(kind: PolicyArg, optimal: &OptimalPolicy) -> Box<dyn Policy> {
    match kind {
        PolicyArg::Optimal => Box::new(optimal.clone()),
        PolicyArg::Random => Box::new(UniformRandomPolicy),
    }
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

fn log_progress(run_dir: Option<&Path>, line: &str) -> Result<(), Box<dyn Error>> {
    eprintln!("{line}");
    if let Some(run_dir) = run_dir {
        let path = run_dir.join("logs/progress.log");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{line}")?;
        file.flush()?;
    }
    Ok(())
}

fn fail_handle(handle: RunHandle, kind: &str, message: String) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = handle.dir() {
        eprintln!("Failed run directory: {}", dir.display());
    }
    let _ = handle.fail(kind, message.clone());
    Err(message.into())
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
