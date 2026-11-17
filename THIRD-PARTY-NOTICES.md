# Third-party notices

The launcher is built with these Rust crates (from crates.io, under their own licences, mostly MIT or Apache-2.0):
serde, serde_json, sha2, hex, ed25519-dalek, thiserror, plist, sysinfo and windows-sys. `cargo tree` lists the full set.

The Java agent (`knoxlife-agent.jar`) includes, each under its own licence, with the licence text in `META-INF/licenses/` inside the jar:

| Component | Licence |
|---|---|
| SpongePowered Mixin (Fabric's `sponge-mixin` 0.17.5) | MIT |
| ASM 9.10.1 | BSD-3-Clause |
| MixinExtras 0.5.4 | MIT |
