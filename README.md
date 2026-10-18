# NeoFFSE

NeoFFSE (Neo Final Fantasy Save Editor) edits Final Fantasy save files.
It ships as a desktop app built with Slint. The app runs on the
`neoffse` Rust library, which other Rust projects can use without the
app. The scope covers Final Fantasy I to XVI, except XIV. PS4, PS5, Switch and Xbox
encrypt their saves: NeoFFSE might reads a console save once you have
extracted and decrypted it with another tool.

## Status

Pre-alpha. Nothing works yet. The first target is Final Fantasy I to VI
Pixel Remaster on Steam PC (Windows and Linux, Proton included). See
[`docs/roadmap.md`](docs/roadmap.md) for the milestones.

## Principles

- Never lose data: fields the editor does not know survive a save.
- Never write without a backup and a read-back check.
- Edit spells, items and levels through dedicated widgets, with the raw
  value in reach.
- One format per isolated, tested module.
- Show every change before saving, with undo and redo.

[Principles](docs/architecture.md#principles) in the architecture
document gives the reason behind each one.

## Workspace

| Crate | Role |
|---|---|
| `neoffse` | public facade of the library, the crate other projects depend on |
| `neoffse-core` | domain model: games, characters, edits, field metadata |
| `neoffse-data` | ID tables per game, from `data/*.toml` |
| `neoffse-io` | save discovery, backups, verified atomic writes |
| `neoffse-format-pr` | FF I to VI Pixel Remaster format |
| `neoffse-app` | Slint desktop app |
| `neoffse-testkit` | test fixtures, not published |

## Build

```sh
cargo build --workspace
./scripts/clippy.sh
cargo test --workspace
```

## License

You may use NeoFFSE under the MIT license or the Apache License 2.0, at
your option. The app uses [Slint](https://slint.dev) under the Slint
Royalty-free Desktop, Mobile and Web Applications License 2.0. NeoFFSE ports
KiameV's MIT-licensed Pixel Remaster editor, whose copyright notices sit
in [`licenses/`](licenses/).

Final Fantasy, its game data and its artwork belong to Square Enix.
NeoFFSE is not affiliated with or endorsed by Square Enix.
