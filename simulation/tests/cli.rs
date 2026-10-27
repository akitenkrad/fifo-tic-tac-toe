use std::process::Command;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fifo-ttt"))
}

#[test]
fn solve_and_show_surfaces_exist() {
    for name in ["solve", "show", "all", "naive", "play"] {
        let output = binary().args([name, "--help"]).output().expect("binary should start");
        assert!(output.status.success(), "{name} --help must succeed");
        let stdout = String::from_utf8(output.stdout).expect("help must be UTF-8");
        assert!(stdout.contains("--scratch"));
        assert!(stdout.contains("--prod"));
    }
}

#[test]
fn solve_requires_an_explicit_run_mode() {
    let output = binary().args(["solve", "--rule", "t3"]).output().expect("binary should start");
    assert!(!output.status.success());
}

#[test]
fn scratch_and_prod_are_mutually_exclusive() {
    let output = binary()
        .args(["show", "--rule", "t3", "--kx", "3", "--ko", "3", "--scratch", "--prod"])
        .output()
        .expect("binary should start");
    assert!(!output.status.success());
}

#[test]
fn phase_two_options_are_exposed() {
    let solve = binary().args(["solve", "--help"]).output().expect("binary should start");
    let solve_help = String::from_utf8(solve.stdout).unwrap();
    assert!(solve_help.contains("--first"));
    assert!(solve_help.contains("--horizon"));

    let naive = binary().args(["naive", "--help"]).output().expect("binary should start");
    let naive_help = String::from_utf8(naive.stdout).unwrap();
    assert!(naive_help.contains("--node-cap"));

    let play = binary().args(["play", "--help"]).output().expect("binary should start");
    let play_help = String::from_utf8(play.stdout).unwrap();
    assert!(play_help.contains("--games"));
    assert!(play_help.contains("--seed"));
    assert!(play_help.contains("--max-plies"));
}
