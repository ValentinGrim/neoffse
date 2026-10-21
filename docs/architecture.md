# NeoFFSE architecture

NeoFFSE is a Rust library that edits Final Fantasy save files, and a desktop app built with Slint on top of it.  
You can depend on the `neoffse` library and edit saves without the app.  
Each save format has its own reference under `docs/formats/`, starting with [`pixel-remaster.md`](formats/pixel-remaster.md).

## Goals and scope

NeoFFSE edits the saves of Final Fantasy I to XVI, XI and XIV excepted, and starts with the Pixel Remaster (PR) of I to VI on Steam PC.

- **Library first.** Any Rust project can edit a save through the `neoffse` crate.
- **First target.** FF I to VI PR, Steam release, on Windows and on Linux (native Steam, Flatpak Steam, Proton).
- **Other platforms.** Per-platform versions might follow the PC ones.
- **Console encryption.** PS4, PS5, Switch and Xbox encrypt their saves, and NeoFFSE does not plan to decrypt them.

### Licensing constraints

- The project code is `MIT OR Apache-2.0`.

- Slint comes under the Slint Royalty-free Desktop, Mobile and Web Applications License 2.0.
  That license requires the `AboutSlint` widget in the app's About dialog, and the user reaches that dialog from the top-level menu.

- The ID tables come from the game's own master data.  
  `tools/pr-tables/` reads the master data and message files of the user's install ([Game master data](formats/pixel-remaster.md#game-master-data))
  and generates the tables under `data/`.

- KiameV's archived, MIT-licensed editor [final-fantasy-pr-save-editor](https://github.com/KiameV/final-fantasy-pr-save-editor)
  is the source of the save-format rules that [`pixel-remaster.md`](formats/pixel-remaster.md) documents.  
  Its MIT notice stays in `licenses/`.

- The Rijndael-256 cipher is a hand port of KiameV's MIT-licensed [ffpr-save-cypher](https://github.com/kiamev/ffpr-save-cypher).  
  That library is based on Meyti's MIT-licensed [py3rijndael](https://github.com/meyt/py3rijndael),  
  whose origin is a gist that ports Bram Cohen's public-domain Java reference code of 2001.  

- The notices of KiameV and Meyti sit in `licenses/`; [`licenses/README.md`](../licenses/README.md) maps each file to the work it covers.

- Contributors may read the format notes of GPL editors (Black Chocobo for FF7, Hyne for FF8) and must not copy their code.  
  Their GPL code would force the GPL on NeoFFSE.

- Square Enix owns the game data and artwork.
  **Graphic and audio assets (sprites, portraits, textures, music, sounds) never enter the repository.**  
  Tables of facts that editing needs (IDs, names, kinds, caps) may, and `tools/pr-tables/` generates them.  
  [Visual assets](#visual-assets) describes how the app shows portraits under that rule.

## Principles

Every crate follows five rules.

1. **Never lose data.** The format layer keeps the original document and applies targeted edits to it, 
   so a field the editor does not know survives a save untouched.  
   KiameV's Go editor shows the risk:  
   it rebuilt the JSON from its own structs, which had no field for `mapData.minimapEnable`, and reset that flag on every save in FF I and FF II.

2. **Never write without a safety net.** A bug in an encoder must cost the player nothing. 
   Before it replaces a file, the editor backs it up with a manifest, writes to a temporary file in the same folder, 
   decodes that file and compares it to the in-memory document, then renames.  
   It also warns when the game is running. 
   The user restores any backup from the Restore page and can undo the restore. [Save safety](#save-safety) lists the steps.
   
3. **Edit concepts through dedicated widgets, and keep the raw value in reach.**  
   A player thinks in spells and levels, and the save stores IDs and integers.
   A character's Magic widget lists the spells it knows and offers a dropdown to add one.
   A spell or skill level shows its named level and its raw value side by side, and both edit the same field.
   An advanced mode shows the raw document tree behind a warning, for the fields no widget covers yet.

4. **One format, one isolated and tested module.** Adding FF VII cannot break FF I.  
   Each format lives in its own crate with its own fixtures and round-trip tests, and depends on no other format crate.

5. **Show what will change before writing.** The user catches a wrong edit before it reaches the disk.
   A change summary opens before every save, and undo and redo cover every edit until then.

## Layers and crates

The engine is a stack of library crates under one facade, `neoffse`.
The desktop app, `neoffse-app`, sits on top and depends on `neoffse` alone.
The facade exports nothing yet: it will re-export the stack (M4), and another Rust project will depend on it the same way.

```
neoffse/
├── Cargo.toml                  workspace root
├── Cargo.lock                  committed, so CI builds the locked versions
├── clippy.toml
├── rustfmt.toml
├── LICENSE-MIT, LICENSE-APACHE
├── README.md
├── scripts/                    clippy.sh, the strict clippy gate
├── crates/
│   ├── neoffse-core/           domain: Game, FieldId, Value, Edit with its inverse,
│   │                           ChangeSet, FieldMeta, Capability, format traits
│   ├── neoffse-data/           ID tables per game, embedded from
│   │                           data/pr/<game>/*.toml
│   ├── neoffse-io/             discovery, verified atomic write,
│   │                           backups, restore, running-game check
│   ├── neoffse-format-pr/      FF I to VI Pixel Remaster: codec,
│   │                           lossless document, game table, slot names.
│   │                           FF I domain mapping and format
│   ├── neoffse/                (M4) public facade: re-exports, format
│   │                           registry, save pipeline
│   ├── neoffse-app/            Slint desktop app
│   │   ├── build.rs
│   │   ├── ui/                 .slint markup
│   │   ├── src/lib.rs          UI layer: window types, callback wiring
│   │   ├── src/main.rs
│   │   └── tests/              offscreen UI tests
│   └── neoffse-testkit/        fixtures and private-save locator, dev-dependency
├── data/                       TOML ID tables
├── fixtures/                   synthetic test saves, committed (pr/)
│                               and private real saves, ignored (private/)
├── tools/cipher-vectors/       Go program that writes the cipher test vectors
├── tools/pr-tables/            Python script that generates data/pr/ from a game install
├── assets/                     (M4) original art, free-license art
├── licenses/                   third-party notices
└── docs/                       this file, roadmap, formats/
```

**Dependency rule.**  
The layering is `neoffse-app → neoffse → neoffse-format-* → neoffse-data → neoffse-core`,
with `neoffse-io` beside the format crates: a crate depends only on crates to its right.  
A format crate knows the domain and its ID tables and never the file system, and `neoffse-io` knows the file system and no format.  

The `neoffse` facade (M4) joins them. The workspace edges are:
- `neoffse-core` and `neoffse-io` depend on no workspace crate.
- `neoffse-data` depends on `neoffse-core`: its tables are keyed by `TableKind` and `Game`.
- `neoffse-format-pr` depends on `neoffse-core` and `neoffse-data`.
- `neoffse` depends on `neoffse-core`, `neoffse-data`, `neoffse-io` and `neoffse-format-pr`.
- `neoffse-app` depends on `neoffse`.
- `neoffse-io` has `neoffse-format-pr` as a dev-dependency, for the ignored `real_discovery` test only.
  It must not become a standard dependency: the I/O crate would then know a format.

A format crate never imports another format crate.
`neoffse-testkit` enters a crate as a dev-dependency and nowhere else.
No crate below the app depends on Slint, so their tests run without opening a window.

**Public types.**  
A public enum that a crate returns, and a struct with public fields that only the crate builds, is `#[non_exhaustive]` (`IoError`, `Value`, `SaveView`).
A later version adds a variant or a field without a breaking change, and a `match` on such an enum needs a wildcard arm.
A type that a caller builds and passes in stays open (`Request`, `Container`).
The metadata and view types come with a constructor (`FieldMeta::new`), so a caller builds one and assigns the public fields.

### Crate roles

- **`neoffse-core`** holds the domain and the traits every format implements.
  It has no format code, no file access and no UI.
  A format names its fields by path (`gil`, `character.1.level`) and describes each one in a `FieldMeta`: label, storage bounds, the game's own cap, named values, referenced ID tables.
  The app lays out its widgets from that metadata.
  An `Edit` returns its inverse when applied, which is how undo works, and a `ChangeSet` groups the edits of one user action.
  A format sees file names and bytes and never a path.

- **`neoffse-data`** holds the ID tables of a game, keyed by `TableKind`, with English and French names and the game's caps (`MAX_GIL` and the like).
  The TOML files under `data/pr/<game>/` hold the text, and `include_str!` embeds it, so a library caller needs no data directory.
  Only FF I has tables so far.
  Names repeat inside a table, so an ID is the only lookup key.
  The parser checks the file contract (ID order, value ranges, placeholder rows), not only the types.
  A contributor regenerates the files by hand with `tools/pr-tables/` ([ID tables](../README.md#id-tables)); neither the build nor the tests run it.

- **`neoffse-io`** touches the user's files and knows no save format.
  It finds save folders, keeps backups, replaces a file through the verified write of [Save safety](#save-safety) and scans for a running game.
  Every file operation goes through the `FileOps` trait, one method per step, so a test makes any single step fail.
  A format crate publishes where its saves sit as plain data (`PrGame` rows in `games::GAMES`), and the caller turns the rows into the descriptors discovery takes.
  Discovery never writes.
  It returns every folder it finds and lists the places it could not read.
  After a OneDrive Known Folder Move, a stale local `Documents` folder and the synced one count as two folders, and the caller chooses.
  The search rules are in [Steam roots and libraries](formats/pixel-remaster.md#steam-roots-and-libraries).

- **`neoffse-format-pr`** reads and writes FF I to VI PR saves in three layers.
  The codec turns a file into JSON text and back, and keeps the BOM, the trailing bytes and the zero padding so that `encode` writes the file as the game did.
  The lossless document reads and edits that text by path and leaves every value it does not touch as written.
  The `ff1` module maps an FF I slot onto the domain of `neoffse-core`.
  `Ff1Format` refuses a file that is no slot, or a slot of another game ([Telling the games apart](formats/pixel-remaster.md#telling-the-games-apart)).
  `Ff1Document` reads and writes by field path, never by position, and reports the capabilities `Party` and `Inventory`.
  The Magic section arrives with M4 ([Roadmap](roadmap.md#m4-app-ff-i-end-to-end)).
  [`docs/formats/pixel-remaster.md`](formats/pixel-remaster.md) holds every byte-level rule.

- **`neoffse`** (M4) re-exports the crates above so a caller depends on one crate.
  It also holds the glue the dependency rule keeps out of the lower crates: the registry that asks each format to detect a file, the edit history, and the save pipeline that joins a format's encoder and decoder to the verified write of `neoffse-io`.

- **`neoffse-app`** is the Slint desktop app.
  Its library target holds the window types that `ui/app.slint` generates and the code that wires their callbacks, and the binary only calls `run`.
  The library target lets integration tests drive the windows (see [Test policy](#test-policy)).

- **`neoffse-testkit`** locates and reads the committed fixtures and finds the private real saves through `NEOFFSE_SAVES`.
  It generates no fixture (see [Test policy](#test-policy)).

### Format traits

A format registers one `SaveFormat` per game.
Opening a save yields a `SaveDocument`, which owns the original data and every edit applied to it.
Both traits live in `neoffse_core::format`; the document trait is dyn-compatible and `Send`, so the app moves a document to a worker thread.

```rust
pub trait SaveFormat: Send + Sync {
    fn detect(&self, probe: &FileProbe<'_>) -> Option<Detection>;
    fn open(&self, files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError>;
}

pub trait SaveDocument: Send {
    fn capabilities(&self) -> &[Capability];
    fn summary(&mut self) -> Result<SaveSummary, FormatError>;
    fn view(&mut self) -> Result<SaveView, FormatError>;
    fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError>; // returns the inverse
    fn encode(&self) -> Result<SaveFiles, FormatError>;
    fn verify(&self, files: &SaveFiles) -> Result<(), FormatError>;
    // plus game(), apply_all(), apply_game(), row_locked(), field_locked()
}
```

- **Files, never paths.** `SaveFiles` carries file names and bytes, so a format cannot reach the disk or choose where a file lands.
  A save that spans several files fits the same type.
- **Detection.** `detect` is pure: it reads a `FileProbe` (name, size, head of the file) and returns a `Detection`, or `None`.
  `Confidence` runs `Possible` (the name fits), `Likely` (the name and a cheap content check fit), `Certain` (the first bytes prove it), so the registry picks the best claim.
  The six PR formats claim the same file names, so the facade (M4) breaks the tie with the game of the save folder.
  A format never claims the leftover `.neoffse-*.tmp` file of an atomic write.
- **Open.** `open` copies what it needs, so the document keeps no borrow.
  It refuses a file that does not fit its game with a `FormatError::Decode`, never a panic.
- **Views.** `summary` feeds the Home screen.
  `view` returns the editable content as a snapshot: one section per capability, each field with its `FieldMeta` and current value.
  Both take `&mut self` because a format may defer parsing, and neither changes a value.
- **Encode and verify.** `encode` returns the files to write under the names `open` received.
  `verify` decodes such files from their bytes alone and compares the result with the whole in-memory document, including the data the view omits.
  The facade (M4) calls it from the hook of `neoffse_io::atomic::write` ([Save safety](#save-safety), step 6).

### Edit interfaces

A document edits through two interfaces over one `Edit` type: `Set` a field, `Insert` a row into a collection, or `Remove` a row.
The UI picks the interface, and the library keeps no mode.
Every `apply` returns the inverse edit, and redo applies the original edit again.

- **Raw: `apply(edit, policy)`.** The edit meets the storage bounds of the field.
  Under `CapPolicy::Enforce` it also meets the game's cap and the ID tables.
  `CapPolicy::StorageOnly` accepts any value the save can hold, which is the app's expert mode.
  The raw interface holds no game rule, so it can write a bare equipment ID or a count below the number of wearers.
  Inverses, rollbacks and redos run under `StorageOnly`, because they restore a value the document already held, and the user may have set it above the cap.
- **Game: `apply_game(edit)`.** The edit changes the save the way the game would, so the result keeps the invariants the game keeps.
  Caps and ID tables are always enforced.
  In FF I, setting `character.<id>.equipment.<slot>` expands into the slot, the inventory row and the slot count ([Equipment slots](formats/pixel-remaster.md#equipment-slots)).
  The call returns a `GameChange`: the plain edits it applied and their inverses, which replay through `apply_all` under `StorageOnly` and cannot fail on a game rule.
  `OutOfRange` names the range the edit broke, which under this interface can be a game floor such as the number of wearers.
- **Locks.** `row_locked` and `field_locked` tell the UI which controls the game interface refuses whatever the value, so it greys them before the click.
  A refusal that depends on the value is an error, not a lock.
  A row the model cannot restore byte for byte after a removal is locked too.
- **All or none.** `apply_all` applies a `ChangeSet` and, when an edit fails, undoes the applied edits newest first, so the document holds all the changes or none.
  When the undo fails too, the result is `EditError::RollbackFailed` and the caller reloads the document.

### Technology stack

The workspace pins every dependency in its `Cargo.toml` files, and this table gives the reason for each choice.
A crate tagged with a milestone enters the workspace with it.

| Need | Crate | Why |
|---|---|---|
| UI | `slint`, `slint-build` | `.slint` markup, translations through `@tr` (FR and EN) |
| UI tests | `i-slint-backend-testing` | headless tests with no window and no event loop |
| File dialogs (M4) | `rfd` | native dialogs, XDG portal on Linux (Flatpak-safe) |
| JSON | `serde`, `serde_json` | the lossless document layer needs key order (`preserve_order`), a number's written text (`arbitrary_precision`) and the bytes of an untouched value (`raw_value`) |
| Deflate, base64 | `flate2` (`rust_backend`), `base64` | raw deflate without a zlib header in pure Rust; standard alphabet with `=` padding |
| ID table text | `toml`, `serde` | each raw row refuses unknown keys, so a column the export tool adds fails the tests until the parser handles it |
| PR cipher | hand-ported Rijndael-256 | the block is 32 bytes and AES fixes it at 16, so AES crates cannot decrypt a PR save |
| ID table export | Python 3.11 or later, `UnityPy` | reads the user's install; only a contributor who regenerates the tables needs it |
| Cipher test vectors | Go program in `tools/cipher-vectors/` | runs KiameV's ffpr-save-cypher; only a contributor who regenerates the vectors needs Go |
| Binary formats (after M5) | `binrw` | the console and older PC formats after the PR |
| Atomic write | `tempfile` | temporary file in the target folder, then rename |
| Data dir | `directories` | locates the backups ([Save safety](#save-safety)) |
| Steam library list | `keyvalues-parser` | reads `libraryfolders.vdf` |
| Running game | `sysinfo` | process scan before a write; its minimum Rust version, 1.95, sets the `rust-version` of the workspace |
| Errors | `thiserror` in libraries, `anyhow` in the app (M4) | [Errors](#errors) |
| Logging (M4) | `tracing` | |
| Tests | `proptest`; `insta` (M4) | property tests on codecs and on the document; snapshots |

The three Slint crates carry an `=` pin to one exact version in `crates/neoffse-app/Cargo.toml`.
`i-slint-backend-testing` is internal to Slint and requires the exact version of `slint`, so a Slint update moves all three together.

Cargo unifies the features of a dependency across a workspace build, and the app's dependency tree turns on features that `neoffse-format-pr` does not request.
For that reason CI runs the tests of `neoffse-format-pr` alone.

## Errors

A library crate returns errors and does not panic on anything read from a file.

- **One enum per failure domain.** A library crate declares `thiserror` enums, for example `CodecError`, `EditError` and `IoError`.
  A format converts its own errors into `FormatError` and keeps them as the source.
  The app (M4) uses `anyhow` and adds context at the boundary.
- **Non-exhaustive.** Every error enum is `#[non_exhaustive]`, so a `match` needs a wildcard arm and a later version may add a variant.
- **Display and source.** `Display` never repeats the source error, so an `anyhow` chain prints each cause once.
  `RestoreFailed` and `RollbackFailed` return their boxed error as `source()`, so a caller downcasts to `Box<IoError>` or `Box<EditError>`; their rustdoc gives the exact form.
- **No panic path on input data.** The workspace lints deny `unwrap_used` and `expect_used`, and a test root allows both, since a test asserts by panicking.
- **Position.** An error carries the position when the code knows it, and its variant names the step that failed: a byte offset in the file for base64, a rendered path such as `userData.ownedCharacterList.target[2].parameter` for the document.
- **A refused edit changes nothing.** The error names the field and the reason, and the document stays as it was, except after `EditError::RollbackFailed`.
  `OutOfRange` carries the range the edit broke, and `AboveCap` carries the game cap, which the UI reads to offer expert mode.
- **Errors hold personal paths.** The `Display` and `Debug` output of the `neoffse-io` errors and of its discovery, backup and environment types name paths under the user's home.
  `Debug` of `SaveFolder` also holds a SteamID64, `ProcessInfo` the first command-line argument of a process, and `Manifest` the source folder.
  An error from `LibraryListInvalid` can quote a line of `libraryfolders.vdf`.
  No function of the crate logs or prints them, and a caller must not print them raw: log the variant and the `Operation`, or a count of skipped places.
- **Save content in `Debug` output.** The types that hold a slot's content (`SaveSummary`, `SaveView`, `Value`, `Edit`, `Document`, `Ff1Model` and their parts) print character names, field values or the JSON text.
  A caller must not log them.
  The rustdoc of each such type says "`Debug` prints save content: never log it."
  `SaveFiles` prints file names and byte counts only, and `FormatError` and `OpenError` hold file names, counts and document paths, never a path on disk.

## Test policy

`cargo test --workspace` passes on a fresh clone without any game file.

- **Tests live in `tests/`.** No source file holds test code, in any crate.
  A crate's tests sit in its `tests/` directory as integration tests and use the public API.
  An internal stage that needs tests, such as a codec step or the cipher, is exposed in a documented public module.
  Helpers that several test files share sit in `tests/common/mod.rs`.
- **Round trip on every fixture.** Decode, encode, decode again: the two decoded documents and their containers must match.
  The game's deflate encoder and `flate2` choose different streams for the same input, so a PR save never re-encodes to the game's bytes, and PR round trips compare the inflated JSON text byte for byte.
  The game accepts a deflate stream that differs from its own.
  On real saves, the round trip also compares the BOM and the trailing whitespace.
- **Edit proof.** An edit, then encode and decode, yields the intended change and no other difference in the JSON text.
  The runner in `crates/neoffse-format-pr/tests/common/edit_proof.rs` drives an edit through `Box<dyn SaveDocument>` and compares the JSON tree before and after.
  It fails on a change it did not expect and on a change it expected and did not find.
  It then undoes the edit, on the edited document and on a reopened one, and applies it again.
  An edit matrix runs on the synthetic slot `fixtures/pr/ff1-slot.json` and on every private FF I slot.
- **Domain contract.** `neoffse-core` tests the edit contract with hand-built documents and `proptest`: a refused edit changes nothing, an inverse restores the document, and a `ChangeSet` applies all or none.
  `neoffse-data` tests the embedded FF I tables against the facts of [Game master data](formats/pixel-remaster.md#game-master-data).
- **Properties.** `proptest` drives each codec stage, the whole pipeline and the document with generated input.
  Decode inverts encode, an integer edit changes only its own digits, and no input makes a stage panic.
  A failing case lands in `tests/*.proptest-regressions`; commit that file with the fix.
- **Cipher vectors.** A Go program in `tools/cipher-vectors/` runs KiameV's ffpr-save-cypher and writes Rijndael-256 vectors to `crates/neoffse-format-pr/tests/data/rijndael256-vectors.txt`.
  The repository commits that file, and the Rust port must reproduce every vector.
  The output depends on the pinned library version and a fixed seed alone.
  To regenerate it, run `go run . -o ../../crates/neoffse-format-pr/tests/data/rijndael256-vectors.txt` from `tools/cipher-vectors/`.
- **Fixtures.** Synthetic fixtures are committed under `fixtures/`, and `neoffse-testkit` locates and reads them.
  A test fails when a committed fixture differs from what its generator writes, for example after a compression update changes the deflate stream.
  The test headers give the regeneration command, which sets `NEOFFSE_WRITE_FIXTURES=1`.
  The FF I slot holds invented values that obey the game's rules, so the proof runs against a consistent save.
- **Real saves.** They are personal data and stay out of the repository.
  A test asks the testkit for the saves of one game, for example `neoffse_testkit::private_saves("ff1")`.
  The testkit reads the folder named by `NEOFFSE_SAVES`, `fixtures/private/` by default.
  When the folder holds nothing for that game, the call returns `None` after it writes the reason on stderr, and the test returns early and passes.
  A wrong `NEOFFSE_SAVES` path skips every real-save test while the run stays green; `cargo test -- --nocapture` shows the reasons.
  The testkit tests its skip path through a child process, because editing the environment of a running test needs `unsafe` in edition 2024 and the workspace denies `unsafe_code`.
  Real-save tests only read, and a failure message names a slot label and a field path, never a value.
- **Real discovery.** `crates/neoffse-io/tests/real_discovery.rs` is `#[ignore]`, because it reads the Steam install of the machine it runs on.
  It checks that discovery finds a save folder for each PR game and that the folder holds a file the codec accepts.
  Its output carries game ids and counts, never a path, a SteamID64 or a file name.
  On Linux, run `cargo test -p neoffse-io --test real_discovery -- --ignored --nocapture`.
  For Windows, build the executable on Linux with `cargo test -p neoffse-io --test real_discovery --target x86_64-pc-windows-gnu --no-run` (this needs `mingw-w64` and the `x86_64-pc-windows-gnu` Rust target), copy `target/x86_64-pc-windows-gnu/debug/deps/real_discovery-<hash>.exe` to the Windows machine and run it there with `--ignored --nocapture`.
- **Test profile.** The root `Cargo.toml` sets `opt-level = 2` in `[profile.test]`, because the cipher and the property tests are CPU bound.
- **UI and snapshots (M4).** View models carry the UI logic and run in plain Rust tests, and `insta` will pin change summaries and view model state.
  The Slint layer runs on Slint's testing backend: its tests call `init_no_event_loop`, so they create no window and no event loop.
  No test opens a window on the developer's desktop.
- **CI.** `.github/workflows/ci.yml` runs on each push to `develop` and to a milestone branch (`v*.*.*`), and on each pull request into `develop`.
  The `checks` job runs on Linux and Windows: format, a `Cargo.lock` check, `scripts/clippy.sh` and `cargo test --workspace --locked`.
  On Linux it also tests `neoffse-format-pr` alone (feature unification can hide a missing feature) and builds the docs with warnings denied, so a broken intra-doc link fails the build.
  The `msrv` job checks the workspace on the `rust-version` toolchain.
  CI leaves `NEOFFSE_SAVES` unset, so tests that need real saves skip there.

## Comment doctrine and source hygiene

A comment states what the code cannot show; it never narrates the next line.

- Comments carry invariants, units and format rules.
  A magic number or a format rule cites its heading in `docs/formats/*.md` by file and anchor, for example `// padding stays in the stream, pixel-remaster.md#zero-padding-and-the-stripping-trap`.
- Delete a comment that repeats the code.
  Write a full paragraph when a block needs one to be safe to touch.
- A comment cites what a reader can still resolve in a year: a `docs/formats` heading, a test name, a milestone number.
  A task or review number means nothing outside the tracker that issued it.
- A Rust source or test file holds 300 lines at most.
  Split it into modules before it gets there.
  The Go program under `tools/` is outside the rule.
- The lint set lives in the root `Cargo.toml` under `[workspace.lints]`, and `scripts/clippy.sh` runs clippy with `-D warnings` over every target.
  An exception lives in the code as `#[allow(..., reason = "...")]`, and every `allow` carries its reason.
  The one exception outside the code is the allowed duplicate `syn` in `clippy.toml`, which explains itself.
- Run `cargo fmt` with the committed `rustfmt.toml` before every commit.

## UI architecture

The `.slint` files draw; Rust view models decide.
The screens below land with M4; M0 ships the window, the top-level menu and the About dialog.

- **Presentation only in `.slint`.** Layout, styling and `@tr` strings.
  No validation, no bounds, no save knowledge.
- **View models.** One Rust view model per screen turns domain types into Slint structs and `VecModel`s, and user actions into `Edit`s, and it runs without a window.
  A Slint callback becomes an `Edit` that the view model sends through the facade, and the view model then refreshes from the document.
  It picks each widget from `FieldMeta` (see [Principles](#principles), rule 3): an ID table gives a searchable dropdown, a named enum gives a dropdown beside a raw spin box, and the bounds clamp the spin box.
- **Default and expert mode.** In the default mode the view model sends a new edit through `apply_game`, keeps the returned `GameChange` for undo and redo, and greys a control that `field_locked` or `row_locked` reports.
  In expert mode it sends the edit through `apply` with `CapPolicy::StorageOnly`.
  An `AboveCap` error from the default mode is the cue to offer the switch ([Edit interfaces](#edit-interfaces)).
- **Threads.** The Steam scan and other slow work run on a worker thread, which returns its result to the UI through `slint::invoke_from_event_loop`.
- **Screens.**
  - *Home*: the detected saves with their summary (game, slot, party, play time), plus an Open button for any file.
  - *Editor*: one section per `Capability` the save reports.
  - *Pending changes bar*: the count of unsaved edits, Undo, and "Review & save", which opens the change summary.
  - *Restore*: the backups of the current game, newest first, with their reason and date.
  - *Advanced*: the raw document tree behind a warning. No milestone schedules it yet.
  - *About*: version, licenses and the `AboutSlint` widget, reached from the top-level menu.
- **Assets.** The app asks an `AssetProvider` for portraits and icons ([Visual assets](#visual-assets)).

## Visual assets

The app (M4) picks each image from the best of three providers, and the repository never holds a graphic or audio asset extracted from a game.
One `AssetProvider` trait in the app layer hides the choice, so no library crate depends on it.
The providers, in order of preference:

1. **Local extraction.** The app extracts portraits from the user's own game installation into the user cache directory (`~/.cache/neoffse/assets/<game>/` on Linux), and NeoFFSE never commits or redistributes them.
   The provider will sit behind the `local-assets` cargo feature of `neoffse-app`, off by default, so public builds leave it out.
   It needs the install folder, which `neoffse-io` discovery does not locate (`NEOFFSE_GAME_DIR` will override the lookup), and the Unity asset bundles of the PR games need research before any code.
   No milestone schedules it.
2. **Bundled free art.** Files in `assets/` with a known author and a free license, attributed per file in REUSE style (`REUSE.toml`).
   The folder holds no such file yet.
3. **Silhouettes (M4).** Original generic images in `assets/`.
   They replace any missing portrait, and public builds show no other images for now.

A user who owns the game sees real portraits on their own machine, and a public build redistributes nothing the project has no right to ship.

## Save safety

A save reaches disk through one pipeline in the `neoffse` facade (M4), and any failure before the final rename leaves the original file as it was.
`neoffse-io` provides the backup, write, rename and process steps (M2), and the facade joins them to the format's encoder and decoder.

1. The facade checks for a running game process through the [`process`](../crates/neoffse-io/src/process.rs) module of `neoffse-io`, and the app warns when it finds one.
   The user may cancel or continue.
2. The user reviews the change summary and confirms.
3. `neoffse-io` copies the current files into a new backup folder and writes its `backup.json`.
4. The format encodes the document.
   The PR encoder refuses a JSON text above `MAX_PAYLOAD_LEN` (16 MiB), the cap the decoder applies, so the pipeline never writes a file that step 6 or a later load would refuse.
5. `neoffse-io` writes the bytes to a temporary file in the save's own folder, so the rename stays on one filesystem, and flushes it to disk.
6. The format decodes the temporary file, and the facade compares the result with the in-memory document (`SaveDocument::verify`, which covers the whole document, not only the view).
   The facade hands this check to `neoffse_io::atomic::write` as a hook.
   On a mismatch, or on any other failure before the rename, a guard in `neoffse-io` deletes the temporary file, also when the hook panics, and the call returns an error.
7. `neoffse-io` renames the temporary file over the original.

Step 3 runs before step 4, so an encode that fails leaves a backup that counts against the retention limit of its game.

**Process matching.**
The scan is best effort: a process the system hides stays invisible, so the check can warn but never prove that the game is closed.
A PR game runs under its Windows executable name (`FINAL FANTASY II.exe`), and on Windows the check compares the process name.
On Linux the game runs under Wine, where the kernel cuts the process name to 15 bytes: FF II, FF III and FF IV all read `FINAL FANTASY I`.
The loader that `/proc/<pid>/exe` names is the same for every Wine process, so it cannot name the game either.
Under Proton, Wine rewrites the first command-line argument to the Windows path of the game executable (measured on FF II).
The check takes the last component of that argument, split on `/` and on `\`, and compares it with the executable name, ASCII case folded.

**Backup layout.**

```
<data dir>/backups/<game>/<YYYYMMDD-HHMMSS>[-NN]/
├── backup.json
└── <the save files, under their original names>
```

Backups hold user data, so they sit in the platform data folder that `neoffse_io::dirs::DataDir` names: `$XDG_DATA_HOME/neoffse` (default `~/.local/share/neoffse`) on Linux, `%APPDATA%\neoffse\data` on Windows.
`<game>` is a game id of lowercase ASCII letters, digits and `-`, such as `ff1-pr`.
The folder name is a UTC timestamp, with a `-NN` suffix for backups taken in the same second.
A backup holds the files its request names, which is the edited slot and not the whole save folder.
The `backup` module documents the naming and the manifest.

- `neoffse-io` keeps the 20 newest readable backups per game and deletes older ones after a backup succeeds.
  The count covers the whole game, not each slot, so edits to one slot can push out the last backup of another.
  A folder with a backup name but no readable `backup.json` is reported by `list` and never counted or deleted.
- A restore reads the manifest and every backup file first, so a missing or unreadable file stops it before any write.
  It then backs up the files it will replace (`"reason": "before-restore"`) and writes the backup's files one by one through the atomic write of steps 5 to 7.
  No format takes part: the check of step 6 compares the bytes of the temporary file with the bytes of the backup.
  The user can undo a restore by restoring the new backup.
- A restore that fails while it writes returns `IoError::RestoreFailed`.
  The files already written keep the backup's version, and no rollback runs, because it would write to the disk that refused the last write.
  The error carries the safety backup as the way back.
- `restore` writes into any existing folder it receives and does not compare it with the `sourceDir` of the manifest, which a library move or a reinstall can make stale.
  The facade (M4) derives `target_dir` from a fresh discovery or from the manifest.
- **Steam Cloud.** Steam syncs PR saves to its cloud.
  After an edit, Steam may report a conflict at the next launch, and the user must keep the local files or Steam replaces the edit with the cloud copy.
  The app (M4) shows this note after a save.

## Layers and milestone boundaries

Each milestone closes one or more crates, and an audit reviews each closed crate before the next milestone opens.
The [roadmap](roadmap.md) holds the scope and exit criteria.

| Milestone | Closes | Audit after |
|---|---|---|
| M0 Foundations | workspace, CI, app skeleton | no |
| M1 PR codec | codec and lossless document of `neoffse-format-pr`; `neoffse-testkit` | yes |
| M2 File safety | `neoffse-io`; the game table and the trailer cap of `neoffse-format-pr` | yes |
| M3 PR domain | `neoffse-core`, `neoffse-data`, the FF I domain mapping and format of `neoffse-format-pr` | yes |
| M4 App, FF I | `neoffse-app` v1, the `neoffse` facade (registry, history, save pipeline) | yes |
| M5 FF II to VI | reopens `neoffse-core`, `neoffse-data`, `neoffse-format-pr` for per-game capabilities | yes |
| Release (v0.7.0) | release packaging, translations | full repository |

## Open points

- **macOS.** CI covers Linux and Windows. macOS builds need a third CI runner and a tester on that system.
- **crates.io.** Publishing `neoffse` and its internal crates on crates.io remains open.
  Every crate sets `publish = false` until that choice, and the manifests carry the metadata `clippy::cargo` requires, so either outcome costs no rework.
  `neoffse-data` embeds `data/pr/` through a path that leaves its crate folder, so a publication first copies the tables into the crate.
