# NeoFFSE

NeoFFSE (Neo Final Fantasy Save Editor) edits Final Fantasy save files.
It ships as a desktop app built with Slint.  
The app runs on the `neoffse` Rust library, which other Rust projects can use without the app.  
The scope covers Final Fantasy I to XVI, except XI and XIV. PS4, PS5, Switch and Xbox encrypt their saves:  
NeoFFSE might read a console save once you have extracted and decrypted it with another tool.

## Status

Pre-alpha.
The app does not open or edit saves yet.  
`neoffse-format-pr` decodes and encodes Pixel Remaster save files and edits their JSON text by path, without rewriting the values it does not touch.  
It maps nothing to a domain model yet: `neoffse-core`, `neoffse-data`, `neoffse-io` and `neoffse` hold no code.  
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

|        Crate        |                                 Role                                  |
|:-------------------:|:---------------------------------------------------------------------:|
| `neoffse`           | public facade of the library, the crate other projects depend on (M4) |
| `neoffse-core`      | domain model: games, characters, edits, field metadata (M3)           |
| `neoffse-data`      | ID tables per game, from `data/*.toml` (M3)                           |
| `neoffse-io`        | save discovery, backups, verified atomic writes (M2)                  |
| `neoffse-format-pr` | FF I to VI Pixel Remaster format: codec and lossless document         |
| `neoffse-app`       | Slint desktop app                                                     |
| `neoffse-testkit`   | locates fixtures and private saves for tests                          |

Every crate sets `publish = false` until a crates.io release is decided.

## Build

NeoFFSE needs Rust 1.92 or newer, the minimum Slint 1.18 sets.

```sh
cargo build -p neoffse-app    # the desktop app
cargo build --workspace       # every crate
cargo test --workspace
./scripts/clippy.sh
cargo fmt --all --check
```

`cargo run -p neoffse-app` opens the app window.

`cargo test --workspace` needs no game file.
Tests on real saves read the folder named by `NEOFFSE_SAVES` (default `fixtures/private/`) and pass without running when it holds nothing, as [Test policy](docs/architecture.md#test-policy) explains.

On Debian and Ubuntu, Slint's build guide lists these packages for the
X11 and Wayland backends:

```sh
sudo apt-get install libfontconfig-dev libxkbcommon-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

## License

You may use NeoFFSE under the MIT license or the Apache License 2.0, at your option.
The app uses [Slint](https://slint.dev) under the Slint Royalty-free Desktop, Mobile and Web Applications License 2.0.
NeoFFSE ports the cipher of KiameV's MIT-licensed ffpr-save-cypher library, which is based on Meyti's MIT-licensed py3rijndael.
The ID tables of KiameV's Pixel Remaster editor will follow in M3.
The copyright notices sit in [`licenses/`](licenses/).

Final Fantasy, its game data and its artwork belong to Square Enix.
NeoFFSE is not affiliated with or endorsed by Square Enix.
