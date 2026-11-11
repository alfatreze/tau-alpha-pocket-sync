//! `cargo run -p tps-core --example rebuild -- <card folder> [--write]`: the plugin's rebuild on a folder (the plan is
//! printed; with `--write` the library and covers are written, no renames).
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(card) = args.first() else {
        eprintln!("usage: rebuild <card folder> [--write]");
        std::process::exit(2);
    };
    let card = Path::new(card);
    match tps_core::library::plan(card) {
        Ok(p) => print!("{}", tps_core::library::render_plan(&p)),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
    if args.iter().any(|a| a == "--write") {
        match tps_core::library::run(card, &mut |_, _| {}, None) {
            Ok(d) => print!("{}", tps_core::library::render_done(&d)),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}
