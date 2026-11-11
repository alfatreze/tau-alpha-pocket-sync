//! `cargo run -p tps-core --example rebuild -- <card folder> [--write]`: the plugin's rebuild on a folder, for the golden
//! comparison against `tools/tau_library.py` (the plan is printed; with `--write` the library is written).
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(card) = args.first() else {
        eprintln!("usage: rebuild <card folder> [--write]");
        std::process::exit(2);
    };
    let card = Path::new(card);
    match tps_core::rebuild::plan(card) {
        Ok(p) => print!("{}", tps_core::rebuild::render_plan(&p)),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
    if args.iter().any(|a| a == "--write") {
        match tps_core::rebuild::run(card) {
            Ok(d) => print!("{}", tps_core::rebuild::render_done(&d)),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}
