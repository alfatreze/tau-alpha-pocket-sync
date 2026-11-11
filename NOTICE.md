# Notices

This project is MIT licensed (see `LICENSE`). A plugin loaded by a host through its documented plugin interface is a separate work: the hosts' own licences (Pocket Sync is AGPL-3.0) do not apply to it, and no code of any host is used here.

**Plugin interface:** the message protocol (`start`, `handle_response`, `Choice` / `Text` / `Exit`, `print_msg`, `open_url`) follows the MIT-licensed reference plugin <https://github.com/openfpga-library/pocket-plugin>. Its code is not copied; the protocol is re-implemented in `crates/tps-plugin`.

**Third-party crates in the shipped `plugin.wasm`** (all permissive; from `cargo metadata`, 2026-11-11): `extism-pdk` and its helper crates (BSD-3-Clause); `serde`, `serde_json`, `anyhow`, `base64`, `either`, `hashbrown`, `itertools`, `itoa`, `memchr`, `num-traits` and the macro helper crates (MIT or Apache-2.0); `prost` (Apache-2.0); `bytes`, `rmp`, `rmp-serde`, `winnow` (MIT); `foldhash` (Zlib); `bytemuck` (Zlib, Apache-2.0 or MIT); `unicode-ident` (MIT or Apache-2.0, and Unicode-3.0). Test-only crates (`extism`, `tempfile`) are not shipped.

This list is updated whenever a dependency is added (the library rebuild will bring in more, for example `deunicode`, BSD-3-Clause).
