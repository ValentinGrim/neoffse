# NeoFFSE

NeoFFSE (Neo Final Fantasy Save Editor) edits Final Fantasy save files.
It ships as a desktop app built with Slint.  
The app runs on the `neoffse` Rust library, which other Rust projects can use without the app.  
The scope covers Final Fantasy I to XVI, except XI and XIV. PS4, PS5, Switch and Xbox encrypt their saves:  
NeoFFSE might read a console save once you have extracted and decrypted it with another tool.

## Status

Pre-alpha.
The app does not open or edit saves yet.  
At the library level, you can read and edit a Final Fantasy I Pixel Remaster slot through the traits of `neoffse-core`.  
`Ff1Format` in `neoffse-format-pr` opens a slot file.
Its document lists the party, the items, the key items and the gil, edits them, and encodes the file back.
The tests show that an edit followed by encode and decode changes the intended values and nothing else.  
A document offers two ways to edit.
`apply_game` edits the way the game does: equipping an item also updates the inventory and the slot count, and a value above the game's cap is refused.
`apply` with `CapPolicy::StorageOnly` writes any value the save can hold, for an expert mode.  
`neoffse-data` embeds the FF I tables: items, equipment, abilities, jobs, commands, monsters and the game's caps, with English and French names.  
`neoffse-format-pr` decodes and encodes the Pixel Remaster save files of the six games and edits their JSON text by path, without rewriting the values it does not touch.
Only FF I has a domain mapping so far.  
`neoffse-io` finds the Steam save folders of the games it is given, replaces a file only after a check passes on the written copy, keeps timestamped backups with a restore, and scans for a running game.  
The game scan is best effort: a process the system hides stays invisible, so an empty result proves nothing.  
The `neoffse` facade holds no code yet.  
The first target is Final Fantasy I to VI Pixel Remaster on Steam PC (Windows and Linux, Proton included).  
See [`docs/roadmap.md`](docs/roadmap.md) for the milestones.

## Principles

- Never lose data: fields the editor does not know survive a save.
- Never write without a backup and a read-back check.
- Edit spells, items and levels through dedicated widgets, with the raw value in reach.
- One format per isolated, tested module.
- Show every change before saving, with undo and redo.

[Principles](docs/architecture.md#principles) in the architecture
document gives the reason behind each one.

## Workspace

|        Crate        |                                    Role                                     |
|:-------------------:|:---------------------------------------------------------------------------:|
| `neoffse`           | public facade of the library, the crate other projects depend on (M4)       |
| `neoffse-core`      | domain model and format traits: games, field paths, edits, field metadata   |
| `neoffse-data`      | FF I ID tables and caps, embedded from `data/pr/ff1/*.toml`                 |
| `neoffse-io`        | save discovery, backups, restore, verified atomic writes, game check        |
| `neoffse-format-pr` | FF I to VI Pixel Remaster: codec, lossless document, game list; FF I format |
| `neoffse-app`       | Slint desktop app                                                           |
| `neoffse-testkit`   | locates fixtures and private saves for tests                                |

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

`cargo run -p neoffse-app` opens the app window.

`cargo test --workspace` needs no game file.
Tests on real saves read the folder named by `NEOFFSE_SAVES` (default `fixtures/private/`) and pass without running when it holds nothing, as [Test policy](docs/architecture.md#test-policy) explains.

On Debian and Ubuntu, Slint's build guide lists these packages for the
X11 and Wayland backends:

```sh
sudo apt-get install libfontconfig-dev libxkbcommon-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

## ID tables

The tables in `data/pr/ff1/` come from the game's own master data and message files.  
The repository commits the generated files, so a build and a test run never need the game.  
To regenerate them, install the game and run `tools/pr-tables/` by hand, in a Python 3.11 or later environment with `requirements.txt` installed:

```sh
cd tools/pr-tables
python3 -m prtables --game ff1 --install "<library>/steamapps/common/FINAL FANTASY PR"
```

The tool only reads your install, and it writes the six TOML files under `data/pr/ff1/` when their content changes.  
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
