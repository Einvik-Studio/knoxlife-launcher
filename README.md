# Knox Life launcher

The launcher for **Knox Life**, a roleplay server for Project Zomboid (Build 42), and the player side of the
Java mods the server uses.

The code is public so you can check exactly what runs on your PC. It is **not** open source: all rights are
reserved (see [LICENSE](LICENSE)).

> Status: in development. Nothing here is released yet.

## What the launcher does when you press Play

1. **Cleans up first.** If the launcher was closed while the game was running (crash, power cut), it puts the game
   files back before doing anything else.
2. **Refuses to patch a running game.** The patch can only be added before Java starts.
3. **Downloads the manifest and checks its signature** (ed25519) against the public key built into the launcher, before
   reading anything in it. The manifest's address is built in, and the launcher never accepts a build older than the
   newest one it has seen.
4. **Checks the game version.** The SHA-256 of your `projectzomboid.jar` must be exactly the one the patch was built
   for. If the game has updated and the patch is not ready, the launcher says so and does not start. There is no way
   around this: a patch that does not match fails silently.
5. **Downloads the patch** to the launcher's own folder (never into the game folder) and checks every file against the
   SHA-256 in the manifest.
6. **Adds the patch** by changing one file, after saving a copy of it:
   - Windows and Linux: `-javaagent:…/knoxlife-agent.jar` first in `vmArgs` in `ProjectZomboid64.json`.
   - macOS: the agent is copied into `Project Zomboid.app/Contents/Java/knoxlife/` and listed first in `JVMOptions` in
     `Contents/Info.plist`.
7. **Starts the game through Steam**, waits for the agent to report that it loaded, and watches the game.
8. **Puts everything back** when the game closes: the copy is written back byte for byte, and the files it added are
   removed.

## What the launcher never does

- Ask for a password or log in to anything. Your identity is your Steam account, which the game already checks.
- Run as administrator.
- Send data about you or your PC anywhere. The error report is only copied when you press "Copy report", and you paste
  it yourself.
- Replace or keep any of the game's files.

To remove everything: uninstall the launcher. The game folder is untouched while the game is not running. "Restore game
files" in Details, or Steam's "Verify integrity of game files", puts back anything left behind.

## Layout

| Folder | What |
|---|---|
| `launcher/kjerne` | The launcher's core, without a window or network: finding the game, the manifest, the version check, the download check, adding and removing the patch, and watching the game |
| `launcher/knoxctl` | The same steps on the command line, for testing and troubleshooting |
| `java/agent` | The Java agent (`knoxlife-agent.jar`): starts Mixin with our Java mods before the game's classes load, writes the stamp the launcher waits for, reads the travel ticket, and makes the anti-cheat report the server checks at login |

Code comments and names are in Norwegian; everything players see is in English.

## What the agent reports at login

The server only lets in players whose game was started by the launcher. The agent looks at the Java the game runs in
and lists what does not belong there: other `-javaagent`, native agents and debuggers, a changed boot class path or
class loader, Java options in environment variables, extra files on the class path, class files in folders on the class
path, and whether tools can attach to the game after it has started. It sends at most 8 findings, each at most 96
characters, with **file names only – never folder paths**, so nothing about your folders or user name leaves your PC.

## Building and testing

```sh
cd launcher
cargo test
cargo build --release -p knoxctl

java/agent/bygg.sh   # JDK 21+, curl and sha256sum (Git Bash on Windows)
```

`bygg.sh` downloads Mixin, MixinExtras and ASM from Maven Central, checks each against the SHA-256 in
`java/agent/libs.lock`, runs the tests, builds `knoxlife-agent.jar`, and finally starts a small stand-in game with the
agent and a test mod to check that Mixin really changes the game's code as it loads. The jar has fixed timestamps and a
fixed file order, so the same code and JDK give the same bytes: compare its SHA-256 with the file the launcher
downloads.

`knoxctl finn` shows where the game is, `knoxctl status` whether the patch is in, and `knoxctl sett-tilbake` puts the
game back.
