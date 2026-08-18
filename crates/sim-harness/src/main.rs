//! `sim-hash` — runs the simulation headless and prints a world hash.
//!
//! Two jobs. It is what the cross-process determinism test drives, proving the core
//! produces identical output in a fresh process rather than merely twice inside one.
//! And it is the shape of the server-side replay verifier described in spec 8.3: feed
//! it a seed and a tick count, compare the fingerprint, no rendering involved.
//!
//!     sim-hash --seed 12345 --ticks 10000 --width 160 --height 120
//!
//! I/O lives here rather than in the core, which has none by design.

use std::process::ExitCode;

use sim_core::{scene, ElementTable};

/// Compiled in rather than read at runtime, so the binary produces the same world from
/// any working directory and the test needs no fixture path.
const ELEMENTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/elements.json"
));

struct Options {
    seed: u64,
    ticks: u64,
    width: u32,
    height: u32,
    census: bool,
    dump: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            seed: 0x4f2a11,
            ticks: 1_000,
            width: 160,
            height: 120,
            census: false,
            dump: false,
        }
    }
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("sim-hash: {message}");
            eprintln!("usage: sim-hash [--seed N] [--ticks N] [--width N] [--height N] [--census] [--dump]");
            return ExitCode::FAILURE;
        }
    };

    let table = match ElementTable::from_json(ELEMENTS_JSON) {
        Ok(table) => table,
        Err(error) => {
            eprintln!("sim-hash: elements.json: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut world = scene::sandbox(options.width, options.height, options.seed, &table);
    world.step_many(options.ticks);

    // Bare hex, one line, nothing else — the test compares stdout verbatim.
    println!("{:016x}", world.hash());

    if options.census {
        for element in table.iter() {
            eprintln!("{:>8}: {}", element.name, world.count_of(element.id));
        }
    }

    if options.dump {
        dump(&world, &table);
    }

    ExitCode::SUCCESS
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);

    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("`{flag}` needs a value"));
        match flag.as_str() {
            "--seed" => options.seed = parse_u64(&value()?)?,
            "--ticks" => options.ticks = parse_u64(&value()?)?,
            "--width" => options.width = parse_u64(&value()?)? as u32,
            "--height" => options.height = parse_u64(&value()?)? as u32,
            "--census" => options.census = true,
            "--dump" => options.dump = true,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }

    if options.width < 3 || options.height < 3 {
        return Err("width and height must be at least 3".to_owned());
    }
    Ok(options)
}

/// Decimal, or hex with an `0x` prefix — world seeds are usually written in hex.
fn parse_u64(text: &str) -> Result<u64, String> {
    let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(digits) => u64::from_str_radix(digits, 16),
        None => text.parse::<u64>(),
    };
    parsed.map_err(|_| format!("`{text}` is not a number"))
}

/// Prints the world as text. There is no renderer yet and will not be one this
/// milestone, so this is the only way to look at what the rules actually do.
fn dump(world: &sim_core::World, table: &ElementTable) {
    for y in 0..world.height() as i32 {
        let mut line = String::with_capacity(world.width() as usize);
        for x in 0..world.width() as i32 {
            let id = world.grid().get(x, y).unwrap_or(sim_core::EMPTY);
            line.push(match table.get(id).map(|element| element.name.as_str()) {
                None => '.',
                Some("wall") => '#',
                Some("sand") => 'o',
                Some("water") => '~',
                // An element with no glyph still shows up, rather than reading as void.
                Some(_) => '?',
            });
        }
        eprintln!("{line}");
    }
}
