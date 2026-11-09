# NeoFFSE

NeoFFSE (Neo Final Fantasy Save Editor) edits Final Fantasy save files.
It ships as a desktop app built with Slint.  
The app runs on the `neoffse` Rust library, which other Rust projects can use without the app.  
The scope covers Final Fantasy I to XVI, except XI and XIV. PS4, PS5, Switch and Xbox encrypt their saves:  
NeoFFSE might read a console save once you have extracted and decrypted it with another tool.

## Status

Pre-alpha.
The app opens, edits and saves Final Fantasy I and Final Fantasy II Pixel Remaster slots on Steam PC (Windows and Linux, Proton included), with undo, a review before each save, backups and restore.
The four other Pixel Remaster games appear in Home and read "Not supported yet".
The [roadmap](docs/roadmap.md) lists the milestones that come next.

## Principles

- Never lose data: fields the editor does not know survive a save.
- Never write without a backup and a read-back check.
- Edit spells, items and levels through dedicated widgets, with the raw value in reach.
- One format per isolated, tested module.
- Show every change before saving, with undo and redo.

The reason behind each one sits in [Principles](docs/architecture.md#principles).

## Workspace

|        Crate        |                                             Role                                             |
|:-------------------:|:--------------------------------------------------------------------------------------------:|
| `neoffse`           | public facade, the crate other projects depend on: registry, session, save pipeline, restore |
| `neoffse-core`      | domain model and format traits: games, field paths, edits, field metadata                    |
| `neoffse-data`      | ID tables and caps of six games, FF I growth and FF II skill tables, from `data/pr/<game>/`  |
| `neoffse-io`        | save discovery, backups, restore, verified atomic writes, game check                         |
| `neoffse-format-pr` | FF I to VI Pixel Remaster: codec, lossless document, game list; FF I and FF II formats       |
| `neoffse-app`       | Slint desktop app                                                                            |
| `neoffse-testkit`   | locates fixtures and private saves for tests                                                 |

Every crate sets `publish = false` until a crates.io release is decided.

## Build

NeoFFSE needs Rust 1.95 or newer, the minimum `sysinfo` 0.39 sets.

```sh
cargo build -p neoffse-app    # the desktop app
cargo build --workspace       # every crate
cargo test --workspace
./scripts/clippy.sh
cargo fmt --all --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

`cargo run -p neoffse-app` opens the app.
Close the game first: a save made while the game runs can be lost or overwritten.
Home lists your saves; open a slot, edit, then press "Review & save" (Ctrl+S) and confirm.
Before a save or a restore replaces a file, the app copies it to `$XDG_DATA_HOME/neoffse/backups/<game>/` (`~/.local/share/neoffse` by default) on Linux or `%APPDATA%\neoffse\data\backups\<game>\` on Windows.
It keeps the 20 newest readable backups per game, and a restore never deletes the backup it reads, so a game can hold 21.
Steam may ask you to keep the local files at the next launch.

`cargo test --workspace` needs no game file.
Tests on real saves read the folder named by `NEOFFSE_SAVES` (default `fixtures/private/`) and pass without running when it holds nothing, as [Test policy](docs/architecture.md#test-policy) explains.

On Debian and Ubuntu, Slint's build guide lists these packages for the
X11 and Wayland backends:

```sh
sudo apt-get install libfontconfig-dev libxkbcommon-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

## ID tables

The tables in `data/pr/<game>/`, one folder per game, come from the game's own master data and message files.  
The repository commits the generated files, so a build and a test run never need the game.  
To regenerate them, install the game and run `tools/pr-tables/` by hand, in a Python 3.11 or later environment with `requirements.txt` installed:

```sh
cd tools/pr-tables
python3 -m prtables --game ff1 --install "<library>/steamapps/common/FINAL FANTASY PR"
```

The tool only reads your install, and `--game` takes `ff1` to `ff6`.  
It writes up to eight TOML files per game under `data/pr/<game>/` when their content changes: seven for FF I, six for FF II and five for FF III to VI.  
It refuses an output folder that lies inside a source folder, and a source folder that lies inside the output folder.  
The tables hold facts the editor needs: IDs, names, kinds and the game's caps.  
They hold no dialogue, no graphic, no audio and no extracted file.

## License

You may use NeoFFSE under the MIT license or the Apache License 2.0, at your option.
The app uses [Slint](https://slint.dev) under the Slint Royalty-free Desktop, Mobile and Web Applications License 2.0.
NeoFFSE ports the cipher of KiameV's MIT-licensed ffpr-save-cypher library, which is based on Meyti's MIT-licensed py3rijndael.
The ID tables come from the game's own master data, which a script in `tools/pr-tables/` reads from your install.
The copyright notices sit in [`licenses/`](licenses/).

Final Fantasy, its game data and its artwork belong to Square Enix.
This repository holds no graphic or audio asset from the games.
NeoFFSE is not affiliated with or endorsed by Square Enix.
