# NeoFFSE roadmap

NeoFFSE should reach its first release, v0.7.0, in six milestones and a release stage:  
foundations, the PR codec, file safety, the PR domain, the app on FF I, FF II to VI, then the release.  
A step closes when its exit checks pass, and the next one opens after it.
[Layers and milestone boundaries](architecture.md#layers-and-milestone-boundaries)
lists the crates each milestone closes.

## M0 Foundations

**Scope.**
- Cargo workspace with the seven crates and the shared lint set.
- CI on Linux and Windows: `cargo fmt --all --check`, a `Cargo.lock` check,
  `scripts/clippy.sh`, `cargo test --workspace --locked`; on Linux also
  the tests of `neoffse-format-pr` alone and `cargo doc` with warnings
  denied; plus a `cargo check` job at the workspace `rust-version`.
- A Slint skeleton window with a top-level menu and an About dialog that
  shows the `AboutSlint` widget.
- `LICENSE-MIT`, `LICENSE-APACHE`, and a `licenses/` folder for
  third-party notices.
- `README.md`, `docs/architecture.md`, this roadmap.

**Exit.** CI passes on both operating systems, and the app opens on
both.

## M1 PR codec

**Scope.**
- A hand port of Rijndael-256 from KiameV's Go implementation. The
  port adds the notices of KiameV and Meyti to `licenses/`.
- Test vectors from the Go implementation, committed, with the Go
  program that generates them (`tools/cipher-vectors/`).
- The codec, decode and encode: BOM, base64 that ignores trailing
  whitespace, decryption that keeps the zero padding, raw inflate that
  stops at the final block, JSON text (see
  [Container encoding](formats/pixel-remaster.md#container-encoding)).
- The lossless JSON document (rule 1 of
  [Principles](architecture.md#principles)).
- [`docs/formats/pixel-remaster.md`](formats/pixel-remaster.md), the
  source of truth for each rule above.
- Library tests on synthetic fixtures, and on private real saves that
  the tests find through `NEOFFSE_SAVES`, with `neoffse-testkit` to
  locate both (see [Test policy](architecture.md#test-policy)).

**Exit.** The Rust cipher reproduces every Go test vector. Real FF I and
FF II saves, read through `NEOFFSE_SAVES`, survive decode, encode and
decode with an identical inflated JSON text.

## M2 File safety

**Scope.** `neoffse-io`:
- Discovery: Steam native, Flatpak and Snap, extra libraries from
  `libraryfolders.vdf`, Proton `compatdata` prefixes, Windows Documents
  including OneDrive.
- Atomic write with verification before the rename (see
  [Save safety](architecture.md#save-safety)).
- Backups with `backup.json`, retention of 20 per game, restore.
- Running-game detection.

`neoffse-format-pr` also holds two pieces for this layer: the table of the six
games (`neoffse_format_pr::games`) that discovery and the process check
consume, and a 64-byte cap on the container trailer.

**Exit.** Failure-injection tests pass: a failed write, verification or
rename at any step leaves the original file unchanged. Discovery finds
the real saves of a Steam install on Linux and on Windows; the ignored
`real_discovery` test checks it (see
[Test policy](architecture.md#test-policy)).

## M3 PR domain

**Scope.**
- The domain model in `neoffse-core`, shared by every game: games, field
  paths, `Edit` with its inverse, `ChangeSet`, `FieldMeta`, `Capability`,
  and the `SaveFormat` and `SaveDocument` traits. A document edits through
  two interfaces: `apply` with a `CapPolicy` (raw, for an expert mode) and
  `apply_game` (the way the game does). See
  [Edit interfaces](architecture.md#edit-interfaces).
- `neoffse-data`, which embeds the TOML ID tables of `data/pr/ff1/`
  (content, abilities, jobs, commands, monsters, the game's caps; English
  and French names). `tools/pr-tables/` generates the files from the
  game's master data, and you run it where the game is installed.
- The mapping from the PR document to the domain for FF I, in
  `neoffse-format-pr`: characters, inventory, key items and gil, with the
  slot and game detection, `Ff1Format` and `Ff1Document`. FF II to VI wait
  for M5.

**Exit.** On FF I, an edit followed by encode and decode yields the
intended change and no other difference (see
[Test policy](architecture.md#test-policy)).

## M4 App, FF I end to end

**Scope.** The `neoffse` facade (format registry, edit history, save
pipeline), the Home screen with detected saves, the FF I editor
sections, the change review, save, the Restore page, the About dialog
(see [UI architecture](architecture.md#ui-architecture)). The FF I
Magic section lists the learned spells and stays read only at first;
writing spells waits for research on the spell slot rules
([Abilities](formats/pixel-remaster.md#abilities)). Portraits use
six original class sprites. Local extraction (see
[Visual assets](architecture.md#visual-assets)) waits for research on
the Unity asset bundles.

**Exit.** A real FF I save edited in the app loads in the game.

## M5 FF II to VI

**Scope.** The per-game systems:
- FF II weapon and skill levels.
- FF III and FF V jobs.
- FF VI espers, rages, blitzes and the other parties.
- The bestiaries.

**Exit.** For each game from FF II to FF VI, a real save edited in the
app loads in the game and shows the edit.

## v0.7.0 release

**Scope.** Polish, French and English translations, user documentation,
and CI release builds: a Linux `tar.xz`, a Windows `zip`, and
`SHA256SUMS`.

**Exit.** CI builds both archives and `SHA256SUMS` from a release tag,
the checksums match, and the app from each archive opens a real save on
its system in French and in English.

## After v0.7.0

A guide on adding a format comes first, so a contributor can add a game without reading the whole codebase.  
More games follow, then the platform variants (GBA, memory cards, NDS, consoles).
