# Third-party notices

The launcher is built with these Rust crates (from crates.io, under their own licences, mostly MIT or Apache-2.0):
serde, serde_json, sha2, hex, ed25519-dalek, thiserror, plist, sysinfo and windows-sys, and for the window Tauri with
its clipboard, opener and single-instance plugins, and ureq. `cargo tree` lists the full set.

The window uses three fonts under the SIL Open Font License 1.1, with the licence texts next to them in
`launcher/app/ui/fonter/`:

| Font | Licence |
|---|---|
| Archivo | SIL OFL 1.1 |
| Inter | SIL OFL 1.1 |
| IBM Plex Mono | SIL OFL 1.1 |

The Java agent (`knoxlife-agent.jar`) includes, each under its own licence, with the licence text in `META-INF/licenses/` inside the jar:

| Component | Licence |
|---|---|
| SpongePowered Mixin (Fabric's `sponge-mixin` 0.17.5) | MIT |
| ASM 9.10.1 | BSD-3-Clause |
| MixinExtras 0.5.4 | MIT |
