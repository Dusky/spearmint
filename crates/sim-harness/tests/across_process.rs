//! The process-boundary half of the determinism proof (spec 10, milestone 1).
//!
//! Running twice inside one process shares an address space, an allocator, and a warm
//! set of lazily initialised statics. A fresh process shares none of that, which is
//! what makes this the test that would actually catch an accidental dependency on the
//! environment — and it is the same mechanism the server will use to verify a replay
//! (spec 8.3), just pointed at a different world.

use std::process::Command;

fn run(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_sim-hash"))
        .args(args)
        .output()
        .expect("sim-hash should run");
    assert!(
        output.status.success(),
        "sim-hash failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("hash output should be utf-8")
}

#[test]
fn separate_processes_agree() {
    let args = [
        "--seed", "0x4f2a11", "--ticks", "10000", "--width", "64", "--height", "48",
    ];
    let first = run(&args);
    let second = run(&args);

    assert_eq!(first.trim(), second.trim(), "two processes disagreed");
    assert_eq!(first.trim().len(), 16, "expected a 16-digit hex hash");
}

/// A different seed must give a different world, or the test above would pass on a
/// constant.
#[test]
fn different_seeds_diverge() {
    let base = ["--ticks", "500", "--width", "64", "--height", "48"];
    let first = run(&[&["--seed", "1"], &base[..]].concat());
    let second = run(&[&["--seed", "2"], &base[..]].concat());
    assert_ne!(first.trim(), second.trim());
}

/// Ticks must actually change the world, or a broken step loop would look deterministic.
#[test]
fn more_ticks_change_the_world() {
    let early = run(&[
        "--seed", "5", "--ticks", "10", "--width", "64", "--height", "48",
    ]);
    let later = run(&[
        "--seed", "5", "--ticks", "500", "--width", "64", "--height", "48",
    ]);
    assert_ne!(early.trim(), later.trim());
}
