# Third-party notices

The launcher is built with these Rust crates (from crates.io, under their own licences, mostly MIT or Apache-2.0):
serde, serde_json, sha2, hex, ed25519-dalek, thiserror, plist, sysinfo and windows-sys. `cargo tree` lists the full set.

The Java agent will include, each under its own licence, with the licence text inside the jar:

| Component | Licence |
|---|---|
| SpongePowered Mixin (Fabric's `sponge-mixin`) | MIT |
| ASM | BSD-3-Clause |
| MixinExtras | MIT |
