# Notices

This project is MIT licensed (see `LICENSE`). A plugin loaded by a host through its documented plugin interface is a separate work: the hosts' own licences (Pocket Sync is AGPL-3.0) do not apply to it, and no code of any host is used here.

**Plugin interface:** the message protocol (`start`, `handle_response`, `Choice` / `Text` / `Exit`, `print_msg`, `open_url`) follows the MIT-licensed reference plugin <https://github.com/openfpga-library/pocket-plugin>. Its code is not copied; the protocol is re-implemented in `crates/tps-plugin`.

**Third-party crates in the shipped `plugin.wasm`** (all permissive; from `cargo metadata`, 2026-11-11): `extism-pdk` and its helper crates (BSD-3-Clause); `serde`, `serde_json`, `anyhow`, `base64`, `either`, `hashbrown`, `itertools`, `itoa`, `memchr`, `num-traits` and the macro helper crates (MIT or Apache-2.0); `prost` (Apache-2.0); `bytes`, `rmp`, `rmp-serde`, `winnow` (MIT); `foldhash` (Zlib); `bytemuck` (Zlib, Apache-2.0 or MIT); `unicode-ident` (MIT or Apache-2.0, and Unicode-3.0). Test-only crates (`extism`, `tempfile`) are not shipped.

**Tau Omega's `tau-core`** (MIT, same author; pinned to the tag `tau-core-wasm-1`, <https://github.com/alfatreze/Tau-Omega>) builds the library index and the name sidecar, so the plugin and Tau Omega write the same format.

**Added by `tau-core` (shipped in `plugin.wasm`):** `symphonia` and its `symphonia-*` codec and format crates are **MPL-2.0** (a file-level licence): they are used unmodified, as ordinary dependencies, and their source is at <https://github.com/pdeljanov/Symphonia> (and on crates.io under the same names, at the versions in `Cargo.lock`). `deunicode` (BSD-3-Clause, transliteration of non-Latin names to ASCII); `unicode-normalization`, `png`, `zune-jpeg`, `resize`, `rgb`, `rqrr`, `zip`, `sha2`, `crc32fast`, `base64` (MIT, Apache-2.0 or Zlib); `zlib-rs` (Zlib). The exact versions and their licences are in `Cargo.lock` and the crates' own manifests (`cargo metadata`).

This list is updated whenever a dependency is added.
