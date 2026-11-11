//! What a plugin release must carry: a `plugin.json` the hosts can read, with a logo that exists at the tag the release
//! is made from. (The hosts load `logo_url` from raw.githubusercontent.com at that tag, so the version in the URL must
//! be this crate's version, and the file must be in the repository.)

use std::path::Path;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn plugin_json_has_a_name_a_description_and_a_logo_at_this_version() {
    let text = std::fs::read_to_string(root().join("crates/tps-plugin/plugin.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(!v["name"].as_str().unwrap().is_empty());
    assert!(v["description"].as_str().unwrap().len() > 40);
    assert!(v["allowed_hosts"].as_array().unwrap().is_empty());
    let url = v["logo_url"].as_str().expect("plugin.json needs a logo_url");
    let want = format!(
        "https://raw.githubusercontent.com/alfatreze/tau-alpha-pocket-sync/v{}/assets/plugin-icon.png",
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(url, want, "the logo URL must point at this version's tag");
}

#[test]
fn the_logo_is_a_square_png_of_a_sensible_size() {
    let png = std::fs::read(root().join("assets/plugin-icon.png")).expect("assets/plugin-icon.png");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let w = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
    assert_eq!(w, h, "the logo must be square");
    assert!((128..=2048).contains(&w), "size {w}");
    assert!(png.len() < 1_000_000, "keep the logo small");
}
