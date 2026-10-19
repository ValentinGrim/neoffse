# NeoFFSE roadmap

NeoFFSE reaches v0.1 in six milestones: foundations, the PR codec, file
safety, the PR domain, the app on FF I, then FF II to VI. A milestone
closes when its exit checks pass, and the next one opens after that.
[Layers and milestone boundaries](architecture.md#layers-and-milestone-boundaries)
lists the crates each milestone closes.

## M0 Foundations

**Scope.**
- Cargo workspace with the seven crates and the shared lint set (done).
- CI on Linux and Windows: `cargo fmt --check`, a `Cargo.lock` check,
  `scripts/clippy.sh`, `cargo test --workspace --locked`; plus a
  `cargo check` job at Rust 1.92, the workspace `rust-version`.
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
  port adds KiameV's copyright notice to `licenses/`.
- Test vectors from the Go implementation, committed.
- The codec, decode and encode: BOM, base64 that ignores a trailing CR
  or LF, decryption, restoration of the trailing zero bytes the
  decryption strips, raw inflate, JSON text (see
  [Container encoding](formats/pixel-remaster.md#container-encoding)).
- The lossless JSON document (rule 1 of
  [Principles](architecture.md#principles)).
- [`docs/formats/pixel-remaster.md`](formats/pixel-remaster.md), the
  source of truth for each rule above.
- Library tests on synthetic fixtures, and on private real saves that
  the tests find through `NEOFFSE_SAVES`.

**Exit.** The Rust cipher reproduces every Go test vector. Real FF I and
FF II saves, read through `NEOFFSE_SAVES`, survive decode, encode and
decode with an identical inflated JSON text.

## M2 File safety

**Scope.** `neoffse-io`:
- Discovery: Steam native and Flatpak, extra libraries from
  `libraryfolders.vdf`, Proton `compatdata` prefixes, Windows Documents
  including OneDrive.
- Atomic write with verification before the rename (see
  [Save safety](architecture.md#save-safety)).
- Backups with `backup.json`, retention of 20 per game, restore.
- Running-game detection.

**Exit.** Failure-injection tests pass: a failed write, verification or
rename at any step leaves the original file unchanged. Discovery finds
the real saves of a Steam install on Linux and on Windows.

## M3 PR domain

**Scope.**
- The domain model for FF I to VI in `neoffse-core`: characters, items,
  spells, `Edit` with its inverse, `ChangeSet`, `FieldMeta`,
  `Capability`.
- TOML ID tables under `data/pr/`, exported from the Go editor's tables
  by a one-off tool.
- The mapping from the PR document to the domain.

**Exit.** On FF I, an edit followed by encode and decode yields the
intended change and no other difference.

## M4 App, FF I end to end

**Scope.** The Home screen with detected saves, the FF I editor
sections, the change review, save, the Restore page, the About dialog
(see [UI architecture](architecture.md#ui-architecture)). Portraits use
the silhouettes. Local extraction (see
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

## v0.1 release

**Scope.** Polish, French and English translations, user documentation,
and CI release builds: a Linux `tar.xz`, a Windows `zip`, and
`SHA256SUMS`.

**Exit.** CI builds both archives and `SHA256SUMS` from a release tag,
the checksums match, and the app from each archive opens a real save on
its system in French and in English.

## After v0.1

A guide on adding a format comes first, so a contributor can add a game without reading the whole codebase.  
TBD: More games follow, then the platform variants (GBA, memory cards, NDS, consoles).
