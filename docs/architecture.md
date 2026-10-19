# NeoFFSE architecture

NeoFFSE is a Rust library that edits Final Fantasy save files, and a desktop app built with Slint on top of it.  
You can depend on the `neoffse` library and edit saves without the app.  
Each save format has its own reference under `docs/formats/`, starting with [`pixel-remaster.md`](formats/pixel-remaster.md).  
This file describes the code around the formats.

## Goals and scope

NeoFFSE's goal is to edit the saves of Final Fantasy I to XVI, XI and XIV excepted,
and starts with the Pixel Remaster (PR) of I to VI on Steam PC.

- **Library first.** Any Rust project can edit a save through the
  `neoffse` crate.
- **First target.** FF I to VI PR, Steam release, on Windows and on
  Linux (native Steam, Flatpak Steam, Proton).
- **Other platforms.** Per-platform versions might follow the PC ones.
- **Console encryption.** PS4, PS5, Switch and Xbox encrypt their saves,
  and NeoFFSE does not plan to decrypt them.

### Licensing constraints

- The project code is `MIT OR Apache-2.0`, the Rust convention.
- Slint comes under the Slint Royalty-free Desktop, Mobile and Web
  Applications License 2.0. That license requires the `AboutSlint`
  widget in the app's About dialog, and the user reaches that dialog
  from the top-level menu.
- The PR codec and the first ID tables derive from KiameV's MIT-licensed
  Go code: the archived editor
  [final-fantasy-pr-save-editor](https://github.com/KiameV/final-fantasy-pr-save-editor)
  and [ffpr-save-cypher](https://github.com/kiamev/ffpr-save-cypher).
- The Rijndael-256 cipher is a hand port of ffpr-save-cypher. That library
  is based on Meyti's MIT-licensed
  [py3rijndael](https://github.com/meyt/py3rijndael), whose origin is a
  gist that ports Bram Cohen's public-domain Java reference code of 2001.
  The same Go library generates the cipher test vectors.
- The notices of KiameV and Meyti sit in `licenses/`, and
  [`licenses/README.md`](../licenses/README.md) maps each file to the
  work it covers.
- Contributors may read the format notes of GPL editors (Black Chocobo
  for FF7, Hyne for FF8) and must not copy their code: copied GPL code
  would force the GPL on NeoFFSE.
- Square Enix owns the game data and artwork, so the repository holds no
  file extracted from a game. [Visual assets](#visual-assets) describes
  how the app shows portraits under that rule.

## Principles

Every crate follows five rules.

1. **Never lose data.** The format layer keeps the original document and
   applies targeted edits to it, so a field the editor does not know
   survives a save untouched. KiameV's Go editor shows the risk: it
   rebuilt the JSON from its own structs, which had no field for
   `mapData.minimapEnable`, and reset that flag on every save in FF I and
   FF II. For PR saves, the document parses a JSON-in-string field
   (`userData`, then `ownedCharacterList` inside it) on first access and
   re-serializes it when an edit touched it, and at no other time. A number keeps its
   original text (`1.0` stays `1.0`), keys keep their order, and the
   game's own spellings (`owendGil`) stay as the game wrote them.
2. **Never write without a safety net.** A bug in an encoder must cost
   the player nothing. Before it replaces a file, the editor backs it up
   with a manifest, writes to a temporary file in the same folder, decodes
   that file and compares it to the in-memory document, then renames.
   It warns when the game is running, since the game may overwrite the
   file on its next save. The user restores any backup from the Restore
   page and can undo the restore. [Save safety](#save-safety) lists the
   steps.
3. **Edit concepts through dedicated widgets, and keep the raw value in
   reach.** A player thinks in spells and levels, and the save stores
   IDs and integers. A character's Magic widget lists the spells it knows
   and offers a dropdown to add one. A spell or skill level shows its
   named level and its raw value side by side, and both edit the same
   field. Field metadata (`FieldMeta`: label, the bounds the game can
   store, an enum of named values, a reference to an ID table) decides
   which widget appears. An advanced mode shows the raw document tree
   behind a warning, for the fields no widget covers yet.
4. **One format, one isolated and tested module.** Adding FF VII cannot
   break FF I. Each format lives in its own crate with its own fixtures
   and round-trip tests, and depends on no other format crate.
5. **Show what will change before writing.** The user catches a wrong
   edit before it reaches the disk. A change summary
   (`Firion: Max HP 300 → 999`) opens before every save, and undo and
   redo cover every edit until then.

## Layers and crates

The engine is a stack of library crates under one facade, `neoffse`. The
desktop app, `neoffse-app`, sits on top and depends on `neoffse` alone.
Another Rust project uses the same facade.

```
neoffse/
├── Cargo.toml                  workspace root, shared lints
├── .github/workflows/          CI: fmt, lock check, clippy, tests, rustdoc, MSRV
├── scripts/                    clippy.sh, the strict clippy gate
├── crates/
│   ├── neoffse-core/           domain: Game, Platform, Character, Item,
│   │                           Spell, Edit with its inverse, ChangeSet,
│   │                           FieldMeta, Capability, the format traits
│   ├── neoffse-data/           ID tables per game, from data/*.toml
│   ├── neoffse-io/             discovery, verified atomic write,
│   │                           backups, restore, game-running check
│   ├── neoffse-format-pr/      FF I to VI Pixel Remaster: codec,
│   │                           lossless document, domain mapping
│   │                           (the mapping lands with M3)
│   ├── neoffse/                public facade: re-exports, format
│   │                           registry, save pipeline
│   ├── neoffse-app/            Slint desktop app
│   │   ├── build.rs            compiles ui/app.slint (style fluent)
│   │   ├── ui/                 .slint markup: windows, menu, dialogs
│   │   ├── src/lib.rs          UI layer: window types, callback wiring
│   │   ├── src/main.rs         binary entry point, calls run()
│   │   └── tests/              offscreen UI tests
│   └── neoffse-testkit/        fixtures and private-save locator,
│                               dev-dependency, never published
├── data/                       TOML ID tables (data/pr/ff1/, ...)
├── fixtures/                   synthetic test saves, committed (pr/)
│                               and private real saves, ignored (private/)
├── tools/cipher-vectors/       Go program that writes the cipher test vectors
├── assets/                     original art, free-license art
├── licenses/                   third-party notices
└── docs/                       this file, roadmap, formats/
```

**Dependency rule.**
`neoffse-app → neoffse → neoffse-format-* → neoffse-core + neoffse-data + neoffse-io`.
`neoffse-data` and `neoffse-io` depend on `neoffse-core` and on nothing
else in the workspace; `neoffse-core` depends on no workspace crate. A
format crate never imports another format crate. `neoffse-testkit`
enters a crate as a dev-dependency and nowhere else. No crate below the
app depends on Slint, so their tests run without opening a window.

### Crate roles

- **`neoffse-core`** holds the domain and the traits every format
  implements. No format code, no file access, no UI. An `Edit` names a
  field and a new value; applying it returns the inverse `Edit`, which
  is how undo works. A `ChangeSet` groups the edits of one user action.
  A `Capability` (party, inventory, magic, bestiary, ...) tells the UI
  which editor sections a save supports.
- **`neoffse-data`** loads the ID tables (items, spells, maps, monsters,
  jobs) from the TOML files under `data/`. The build embeds them, so a
  library caller needs no data directory. Names and IDs only.
- **`neoffse-io`** touches the user's files. It finds save roots (Steam
  native and Flatpak, extra libraries from `libraryfolders.vdf`, Proton
  `compatdata` prefixes, Windows Documents including OneDrive), writes
  through the verified atomic pipeline, manages backups and their
  `backup.json` manifest, restores, and detects a running game. Each
  format crate declares where its saves sit relative to those roots.
- **`neoffse-format-pr`** reads and writes FF I to VI PR saves. The codec
  splits off the BOM and the trailing whitespace, decodes base64,
  decrypts with Rijndael-256, inflates the raw deflate stream and returns
  the JSON text. The decrypted stream keeps its zero padding, because the
  inflater stops at the final block and so ignores it, and the codec
  records the BOM and the exact trailing bytes so that `encode` writes
  them back. The lossless document reads and edits that text by path. A
  mapping layer will expose the document as domain types and `FieldMeta`
  (M3).
  [`docs/formats/pixel-remaster.md`](formats/pixel-remaster.md) holds
  every byte-level rule, with the codec layers under
  [Container encoding](formats/pixel-remaster.md#container-encoding).
- **`neoffse`** re-exports the crates above so a caller depends on one
  crate. It also holds the glue the dependency rule keeps out of the
  lower crates: the registry that asks each format to detect a file, the
  edit history, and the save pipeline that joins a format's encoder and
  decoder to `neoffse-io`'s verified write.
- **`neoffse-app`** is the Slint desktop app. Its library target
  (`src/lib.rs`) holds the window types that `ui/app.slint` generates
  and the code that wires their callbacks; the `neoffse-app` binary
  (`src/main.rs`) only calls `run`. The library target lets integration
  tests drive the windows (see [Test policy](#test-policy)).
- **`neoffse-testkit`** locates and reads the committed fixtures, finds
  the private real saves through `NEOFFSE_SAVES`, and reports a skipped
  test on stderr. It generates no fixture (see
  [Test policy](#test-policy)).

### Format traits

A format registers one `SaveFormat`. Opening a save yields a
`SaveDocument`, which owns the original data and every edit applied to
it. The sketch below shows the shape; the real signatures land with M3.

```rust
// Illustrative, not the final API.
pub trait SaveFormat: Send + Sync {
    fn id(&self) -> FormatId;
    fn detect(&self, probe: &FileProbe) -> Option<Detection>;
    fn open(&self, files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError>;
}

pub trait SaveDocument {
    fn game(&self) -> Game;
    fn summary(&self) -> SaveSummary;
    fn capabilities(&self) -> &[Capability];
    fn view(&self) -> SaveView;
    fn apply(&mut self, edit: &Edit) -> Result<Edit, EditError>; // returns the inverse edit
    fn encode(&self) -> Result<EncodedFiles, FormatError>;
}
```

`SaveFiles` and `EncodedFiles` carry several files because some saves
span more than one (a memory card holds many saves, some PC games split
a slot across files). `detect` reads a `FileProbe` (path, size, first
bytes) and returns a confidence, so the registry picks the best match
when two formats could claim a file.

### Technology stack

The workspace uses the crates below. Each `Cargo.toml` pins the version
when code first uses the crate, so this table names no version. A new
dependency outside this table gets a row here first.

The three Slint crates (`slint`, `slint-build`, `i-slint-backend-testing`)
are the exception: `crates/neoffse-app/Cargo.toml` pins each with `=` to
the same exact version. `i-slint-backend-testing` is an internal Slint
crate and requires the exact version of `slint`, so a Slint update moves
all three together.

| Need | Crate | Notes |
|---|---|---|
| UI | `slint` | `.slint` markup, translations through `@tr`, FR and EN |
| UI build | `slint-build` | build script of `neoffse-app` (`build.rs`) compiles `ui/app.slint` with the `fluent` style |
| UI tests | `i-slint-backend-testing` | dev-dependency of `neoffse-app`: headless UI tests with no window and no event loop. Internal Slint crate |
| File dialogs | `rfd` | native dialogs, XDG portal on Linux (Flatpak-safe) |
| JSON | `serde`, `serde_json` | `serde` with `std` only. `serde_json` with `preserve_order` (key order), `arbitrary_precision` (a number keeps its written text) and `raw_value` (an untouched value keeps its bytes). They sit under a lossless document layer |
| Deflate | `flate2` (`rust_backend`) | raw deflate, no zlib header, pure Rust |
| Base64 | `base64` (`std` only) | standard alphabet, `=` padding |
| PR cipher | hand-ported Rijndael-256 | 32-byte block, 32-byte key, CBC, fixed key and IV, zero padding; AES fixes the block at 16 bytes, so AES crates cannot decrypt it. The port computes its S-box and lookup tables by `const fn` and expands the key schedule in `Rijndael256::new` |
| Cipher test vectors | Go program in `tools/cipher-vectors/` | runs KiameV's ffpr-save-cypher (version pinned in its `go.mod`). Only a contributor who regenerates the vectors needs Go |
| Binary formats | `binrw` | for the console and older PC formats after the PR |
| Atomic write | `tempfile` | temp file in the target folder, then rename |
| Config and cache dirs | `directories` | |
| Steam library list | `keyvalues-parser` | reads `libraryfolders.vdf` |
| Running game | `sysinfo` | process scan before a write |
| Errors | `thiserror` in libraries, `anyhow` in the app | [Errors](#errors) |
| Logging | `tracing` | |
| Tests | `insta`, `proptest` (`std` only) | snapshots, property tests on codecs and on the document |

## Errors

A library crate returns errors; it does not panic on anything read from
a file.

- One `thiserror` enum per failure domain of a library crate. Today
  `neoffse-format-pr` has `CodecError` (the container steps) and
  `DocumentError` (reads and edits of the JSON document). M3 adds
  `FormatError` in `neoffse-core`, next to the `SaveFormat` trait; a
  format crate converts its own errors into it and keeps them as the
  source. `EditError` and `IoError` follow with their crates. The app
  uses `anyhow` and adds context at the boundary.
- No panic path on input data. The workspace lints deny `unwrap_used`
  and `expect_used`; a test root allows both, since a test asserts by
  panicking.
- An error carries the position when the code knows it, and its variant
  names the step that failed. `CodecError::Base64` holds the byte offset
  in the file, `CodecError::Inflate` the offset in the decrypted stream.
  `DocumentError` holds a rendered path, for example
  `userData.ownedCharacterList.target[2].parameter`: keys join with dots,
  an index shows in brackets, and the step into a string's document shows
  nothing.
- A refused edit returns an error with the field and the reason, and the
  document stays as it was before the call. `Document::set` already
  behaves so with `DocumentError`; `EditError` will carry the bound an
  edit broke (M3).

## Test policy

`cargo test --workspace` passes on a fresh clone without any game file.

- **Tests live in `tests/`.** No source file holds test code, in any
  crate. A crate's tests sit in its `tests/` directory as integration
  tests and use the public API. An internal stage that needs tests, such
  as a codec step or the cipher, is exposed in a documented public
  module. Helpers that several test files share sit in
  `tests/common/mod.rs`, which Cargo does not build as a test target.
- **Round trip on every fixture.** Decode, encode, decode again: the two
  decoded documents must match. The game's deflate encoder and `flate2`
  choose different streams for the same input, so a PR save never
  re-encodes to the game's bytes, and PR round trips compare the
  inflated JSON text byte for byte. The game accepts a deflate stream
  that differs from its own: FF I loads a bestiary file the Go editor
  re-encoded.
- **Edit tests.** An edit, then encode and decode, yields the intended
  change and no other difference in the JSON text.
- **Properties.** `proptest` drives each codec stage (framing, base64,
  the cipher with its padding, deflate), the whole pipeline and the
  document with generated input. Decode inverts encode, an integer edit
  changes only its own digits, and no input makes a stage panic.
- **Cipher vectors.** A Go program in `tools/cipher-vectors/` runs
  KiameV's ffpr-save-cypher and writes Rijndael-256 vectors (single
  blocks, CBC encryption, CBC decryption with the length the Go `Decrypt`
  returns) to `crates/neoffse-format-pr/tests/data/rijndael256-vectors.txt`.
  The repository commits that file and the Rust port must reproduce every
  vector. The output depends only on the pinned library version and a
  fixed seed. To regenerate it, run, with Go installed:
  `cd tools/cipher-vectors && go run . -o ../../crates/neoffse-format-pr/tests/data/rijndael256-vectors.txt`.
- **Fixtures.** Synthetic fixtures are committed under `fixtures/`, and
  `neoffse-testkit` locates and reads them. The six container fixtures
  of `fixtures/pr/` come from one test; to regenerate them, run
  `NEOFFSE_WRITE_FIXTURES=1 cargo test -p neoffse-format-pr --test container_fixtures write_fixtures`.
- **Real saves.** They are personal data and stay out of the repository.
  A test asks the testkit for the saves of one game, for example
  `neoffse_testkit::private_saves("ff1")`. The testkit reads the folder
  named by `NEOFFSE_SAVES`, `fixtures/private/` by default. When it holds
  nothing for that game, the call returns `None` after
  `neoffse_testkit::skip` writes the reason on stderr; the test returns
  early and passes. `cargo test -- --nocapture` shows the reason. The PR
  tests ignore `steam_autocloud.vdf`, the marker Steam writes beside
  saves, and treat a game folder that holds only that file as absent.
  Real-save tests only read.
- **Snapshots.** `insta` pins change summaries and view model state.
- **UI.** View models carry the UI logic and run in plain Rust tests.
  The Slint layer runs on Slint's testing backend, through
  `i-slint-backend-testing`: its tests call `init_no_event_loop`, so
  they create no window and no event loop. No test opens a window on the
  developer's desktop.
- **Platforms.** CI (`.github/workflows/ci.yml`) runs on each push to
  `develop` and to a milestone branch (`v*.*.*`), and on each pull
  request into `develop`. The `checks` job runs on Linux and on Windows:
  `cargo fmt --all --check`, `cargo fetch --locked` (fails when
  `Cargo.lock` no longer matches the manifests), `scripts/clippy.sh`,
  then `cargo test --workspace --locked`. On Linux, it also runs
  `cargo doc --workspace --no-deps --locked` with
  `RUSTDOCFLAGS=-D warnings`, so a broken intra-doc link fails the build.
  The `msrv` job runs `cargo check --workspace --all-targets --locked`
  on Linux with Rust 1.92.0, the `rust-version` of the workspace. CI
  leaves `NEOFFSE_SAVES` unset, so tests that need real saves skip there.

## Comment doctrine and source hygiene

A comment states what the code cannot show; it never narrates the next
line.

- Comments carry invariants, units and format rules. A magic number or
  a format rule cites its heading in `docs/formats/*.md` by file and
  anchor, for example
  `// padding stays in the stream, pixel-remaster.md#zero-padding-and-the-stripping-trap`.
- Delete a comment that repeats the code. Write a full paragraph when a
  block needs one to be safe to touch.
- A comment cites what a reader can still resolve in a year: a
  `docs/formats` heading, a test name, a milestone number. A task or
  review number means nothing outside the tracker that issued it.
- A source or test file holds 300 lines at most. Split it into modules
  before it gets there.
- The lint set lives in the root `Cargo.toml` under `[workspace.lints]`:
  clippy `pedantic` and `cargo` as groups, `unwrap_used`, `expect_used`,
  `print_stdout` and `print_stderr` by name, and rustc's `missing_docs`,
  `missing_debug_implementations`, `unreachable_pub` and `unsafe_code`,
  all deny. `scripts/clippy.sh` runs clippy with `-D warnings` over
  every target. An exception lives in the code as
  `#[allow(..., reason = "...")]`, never as a script flag.
- Run `cargo fmt` with the committed `rustfmt.toml` (`max_width = 160`)
  before every commit.

## UI architecture

The `.slint` files draw; Rust view models decide. A view model turns
domain data into Slint models and turns user actions into `Edit`s, and
it runs without a window.

- **Presentation only in `.slint`.** Layout, styling and `@tr`
  strings. No validation, no bounds, no save knowledge.
- **View models.** One Rust view model per screen translates domain
  types into Slint structs and `VecModel`s, and back. A Slint callback
  turns into an `Edit` the view model sends through the facade, and the
  view model then refreshes from the document. The view model picks
  widgets from `FieldMeta` (see [Principles](#principles), rule 3): an
  ID table gives a searchable dropdown, a named enum gives a dropdown
  beside a raw spin box, bounds clamp the spin box.
- **Threads.** The Steam scan and other slow work run on a worker
  thread, which returns its result to the UI through
  `slint::invoke_from_event_loop`.
- **Screens.**
  - *Home*: the detected saves, each with the summary read from the file
    (game, slot, party, play time), plus an Open button for any file.
  - *Editor*: one section per `Capability` the save reports.
  - *Pending changes bar*: the count of unsaved edits, Undo, and
    "Review & save", which opens the change summary.
  - *Restore*: the backups of the current game, newest first, with their
    reason and date.
  - *Advanced*: the raw document tree, behind a warning.
  - *About*: version, licenses, and the `AboutSlint` widget, reached
    from the top-level menu.
- **Assets.** The app asks an `AssetProvider` for portraits and icons;
  [Visual assets](#visual-assets) describes the providers.

## Visual assets

The app picks each image from the best of three providers, and the
repository never holds an asset extracted from a game. One `AssetProvider` trait in the app layer hides the
choice, so no library crate depends on it. The providers, in order of
preference:

1. **Local extraction.** The app extracts portraits from the user's own
   game installation into the user cache directory
   (`~/.cache/neoffse/assets/<game>/` on Linux). NeoFFSE never commits
   or redistributes these files. The provider sits behind the
   `local-assets` cargo feature of `neoffse-app`, off by default, so
   public builds leave it out. Save discovery in `neoffse-io` locates
   the game install, and `NEOFFSE_GAME_DIR` overrides it. The Pixel
   Remaster games store their art in Unity asset bundles, and the
   provider needs research on that format before any code.
2. **Bundled free art.** Files in `assets/` with a known author and a
   free license, attributed per file in REUSE style (`REUSE.toml`), the
   way ff7tk does for Black Chocobo. The folder holds no such file yet.
3. **Silhouettes.** Original generic images in `assets/`. They replace
   any missing portrait, and public builds show no other images for now.

A user who owns the game sees real portraits on their own machine, and a
public build redistributes nothing the project has no right to ship.

## Save safety

A save reaches disk through one pipeline in the `neoffse` facade, and
any failure before the final rename leaves the original file as it was.

1. The facade checks for a running game process, and the app warns when
   it finds one. The user may cancel or continue.
2. The user reviews the change summary and confirms.
3. `neoffse-io` copies the current files into a new backup folder and
   writes its `backup.json`.
4. The format encodes the document.
5. `neoffse-io` writes the bytes to a temporary file in the save's own
   folder, so the rename stays on one filesystem, and flushes it to
   disk.
6. The format decodes the temporary file, and the facade compares the
   result with the in-memory document. On a mismatch, the facade deletes
   the temporary file and reports an error.
7. `neoffse-io` renames the temporary file over the original.

**Backup layout.**

```
<config dir>/backups/<game>/<YYYYMMDD-HHMMSS>[-NN]/
├── backup.json
└── <the save files, under their original names>
```

`-NN` separates two backups taken in the same second. The manifest
(illustrative values):

```json
{
  "version": 1,
  "game": "ff1-pr",
  "sourceDir": "/home/user/.local/share/Steam/steamapps/compatdata/<appid>/pfx/drive_c/users/steamuser/Documents/My Games/...",
  "created": "2026-10-18T22:14:03+02:00",
  "files": ["<slot file>"],
  "saveType": "slot",
  "reason": "before-save"
}
```

- `neoffse-io` keeps the 20 newest backups per game and deletes older
  ones after a backup succeeds.
- A restore backs up the current files first (`"reason":
  "before-restore"`), then writes the backup's files through steps 4 to
  7, so the user can undo a restore by restoring that new backup.
- **Steam Cloud.** Steam syncs PR saves to its cloud. After an edit,
  Steam may report a conflict at the next launch; the user must keep the
  local files, or Steam replaces the edit with the cloud copy. The app
  shows this note after a save.

## Layers and milestone boundaries

Each milestone closes one or more crates, and an audit reviews each
closed crate before the next milestone opens. The
[roadmap](roadmap.md) holds the scope and exit criteria.

| Milestone | Closes | Audit after |
|---|---|---|
| M0 Foundations | workspace, CI, app skeleton | no |
| M1 PR codec | codec and lossless document of `neoffse-format-pr`; `neoffse-testkit` | yes |
| M2 File safety | `neoffse-io` | yes |
| M3 PR domain | `neoffse-core`, `neoffse-data`, domain mapping of `neoffse-format-pr` | yes |
| M4 App, FF I | `neoffse-app` v1, the `neoffse` facade (registry, history, save pipeline) | yes |
| M5 FF II to VI | reopens `neoffse-core`, `neoffse-data`, `neoffse-format-pr` for per-game capabilities | yes |
| v0.1 | release packaging, translations | full repository |

## Open points

- **Localized game names.** The UI ships FR and EN, and the ID tables
  carry English names. A French item list needs a source the project
  may use. v0.1 needs it.
- **macOS.** CI covers Linux and Windows. macOS builds need a third CI
  runner and a tester on that system.
- **crates.io.** Publishing `neoffse` and its internal crates on
  crates.io remains open. The manifests carry the metadata
  `clippy::cargo` requires, so either choice costs no rework.
