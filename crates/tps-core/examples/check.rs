//! `cargo run -p tps-core --example check -- <card folder>`: prints the Check text for a folder.
fn main() {
    let card = std::env::args().nth(1).expect("usage: check <card folder>");
    let r = tps_core::scan(std::path::Path::new(&card), tps_core::Limits::default());
    print!("{}", tps_core::report::render(&r));
}
