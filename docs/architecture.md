# NeoFFSE architecture

NeoFFSE is a Rust library that edits Final Fantasy save files, and a desktop app built with Slint on top of it.  
You can depend on the `neoffse` library and edit saves without the app.  
Each save format has its own reference under `docs/formats/`, starting with [`pixel-remaster.md`](formats/pixel-remaster.md).

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
- The ID tables come from the game's own master data.  
  `tools/pr-tables/` reads the master data and message files of the user's install ([Game master data](formats/pixel-remaster.md#game-master-data)) and generates the tables under `data/`.
- KiameV's archived, MIT-licensed editor
  [final-fantasy-pr-save-editor](https://github.com/KiameV/final-fantasy-pr-save-editor)
  is the source of the save-format rules that
  [`pixel-remaster.md`](formats/pixel-remaster.md) documents. Its MIT
  notice stays in `licenses/`.
- The Rijndael-256 cipher is a hand port of KiameV's MIT-licensed
  [ffpr-save-cypher](https://github.com/kiamev/ffpr-save-cypher).  
  That library is based on Meyti's MIT-licensed
  [py3rijndael](https://github.com/meyt/py3rijndael), whose origin is a gist that ports Bram Cohen's public-domain Java reference code of 2001.  
  The same Go library generates the cipher test vectors.
- The notices of KiameV and Meyti sit in `licenses/`, and [`licenses/README.md`](../licenses/README.md) maps each file to the work it covers.
- Contributors may read the format notes of GPL editors (Black Chocobo
  for FF7, Hyne for FF8) and must not copy their code: copied GPL code
  would force the GPL on NeoFFSE.
- Square Enix owns the game data and artwork. Graphic and audio assets (sprites, portraits, textures, music, sounds) never enter the repository.  
  Tables of facts that editing needs (IDs, names, kinds, caps) may, and `tools/pr-tables/` generates them.  
  Dialogue, descriptions and whole extracted files never do.  
  [Visual assets](#visual-assets) describes how the app shows portraits under that rule.

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
   field. Field metadata (`FieldMeta`: label, shape with the storage bounds
   the save can hold, read-only flag, the game's own cap, named values, the
   ID tables the field references) decides which widget appears. An advanced mode shows the raw document tree
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
The facade exports nothing yet: it will re-export the stack (M4), and another
Rust project will depend on it the same way.

```
neoffse/
├── Cargo.toml                  workspace root, shared lints
├── Cargo.lock                  committed, so CI builds the locked versions
├── clippy.toml                 clippy settings (project names for doc_markdown,
│                               the one allowed duplicate crate)
├── rustfmt.toml                rustfmt settings (max_width = 160)
├── .gitattributes              LF line endings, binary fixtures
├── .gitignore                  build output, private saves, local tool files
├── LICENSE-MIT, LICENSE-APACHE
├── README.md
├── .github/workflows/          CI: fmt, lock check, clippy, tests,
│                               format crate alone, rustdoc, MSRV
├── scripts/                    clippy.sh, the strict clippy gate
├── crates/
│   ├── neoffse-core/           domain: Game, FieldId, Value, Edit with
│   │                           its inverse, ChangeSet, FieldMeta,
│   │                           Capability, the format traits
│   ├── neoffse-data/           ID tables per game, embedded from
│   │                           data/pr/<game>/*.toml
│   ├── neoffse-io/             discovery, verified atomic write,
│   │                           backups, restore, running-game check
│   ├── neoffse-format-pr/      FF I to VI Pixel Remaster: codec,
│   │                           lossless document, game table, slot
│   │                           names; the FF I domain mapping and
│   │                           format
│   ├── neoffse/                (M4) public facade: re-exports, format
│   │                           registry, save pipeline
│   ├── neoffse-app/            Slint desktop app: window, menu and About
│   │   │                       dialog today, the screens with M4
│   │   ├── build.rs            compiles ui/app.slint (style fluent)
│   │   ├── ui/                 .slint markup: windows, menu, dialogs
│   │   ├── src/lib.rs          UI layer: window types, callback wiring
│   │   ├── src/main.rs         binary entry point, calls run()
│   │   └── tests/              offscreen UI tests
│   └── neoffse-testkit/        fixtures and private-save locator,
│                               dev-dependency
├── data/                       TOML ID tables (data/pr/ff1/, ...)
├── fixtures/                   synthetic test saves, committed (pr/)
│                               and private real saves, ignored (private/)
├── tools/cipher-vectors/       Go program that writes the cipher test vectors
├── tools/pr-tables/            Python script that generates data/pr/
│                               from a game install
├── assets/                     (M4) original art, free-license art
├── licenses/                   third-party notices
└── docs/                       this file, roadmap, formats/
```

**Dependency rule.**
The layering is
`neoffse-app → neoffse → neoffse-format-* → neoffse-data → neoffse-core`,
with `neoffse-io` beside the format crates: a crate depends only on crates
to its right. A format crate knows the domain and its ID tables and never
the file system, and `neoffse-io` knows the file system and no format. The
`neoffse` facade (M4) joins them. The workspace edges are:

- `neoffse-core` and `neoffse-io` depend on no workspace crate.
- `neoffse-data` depends on `neoffse-core`: its tables are keyed by
  `TableKind` and `Game`.
- `neoffse-format-pr` depends on `neoffse-core` and `neoffse-data`. It has
  no dependency on `neoffse-io`.
- `neoffse` depends on `neoffse-core`, `neoffse-data`, `neoffse-io` and
  `neoffse-format-pr`.
- `neoffse-app` depends on `neoffse`.
- `neoffse-io` has `neoffse-format-pr` as a dev-dependency, for the
  ignored `real_discovery` test only. It is the one edge that points up.
  It cannot become a normal dependency, because it would reverse the
  layering and give the I/O crate knowledge of a format.

A format crate never imports another format crate. `neoffse-testkit`
enters a crate as a dev-dependency and nowhere else. No crate below the
app depends on Slint, so their tests run without opening a window.

**Public types.** Every public enum that a crate returns, and every
struct with public fields that only the crate builds, is
`#[non_exhaustive]`. A later version adds a variant or a field without a
breaking change, and a `match` on such an enum needs a wildcard arm.
`neoffse-io` applies the rule to `IoError`, `Operation`, `Origin`, `Key`,
`Backup`, `Manifest`, `Listing`, `Restored`, `Discovery` and
`SaveFolder`; `neoffse-format-pr` applies it to `CodecError`,
`DocumentError`, `OpenError`, `InflatePath`, `Decoded`, `Framing`,
`Inflated`, `UnalignedLength`, `PrGame` and `SlotFile`; `neoffse-core`
applies it to `Game`, `Value`, `ValueKind`, `Edit`, `GameChange`,
`EditError`, `FormatError`, `Capability`, `TableKind`, `CapPolicy`,
`Shape`, `FieldMeta`, `RowField`, `CollectionMeta`, `FileProbe`,
`Confidence`, `Detection`, `SaveSlot`, `SaveSummary`, `SaveView`,
`Section`, `Entity` and `FieldView`; `neoffse-data` applies it to
`Language`, `AbilityKind`, `AbilityInfo`, `Entry` and `DataError`. A type
that a caller builds and passes in stays open, so the caller can construct
it: `Request`, `Environment`, `SteamGame` and `ProcessInfo` of `neoffse-io`;
`Container`, `Segment` and `Scalar` of `neoffse-format-pr`; `Key` and
`NamedValue` of `neoffse-core`; `Sources` of `neoffse-data`. The metadata and view types are
`#[non_exhaustive]` and come with a constructor (`FieldMeta::new`,
`SaveView::new`), so a caller builds one and then assigns the public fields.

### Crate roles

- **`neoffse-core`** holds the domain and the traits every format
  implements. No format code, no file access, no UI. Ten modules split it:
  - `game` holds `Game`, one variant per title and edition (`Ff1Pr` to
    `Ff6Pr`), with the stable id `ff1-pr` to `ff6-pr`.
  - `field` holds `FieldId`, the path of one field (`gil`,
    `character.1.level`, `inventory.7.count`), `RowId`, the path of one
    row of a collection, and `Value`, the integer, flag or text an edit
    writes. A format declares its own paths and the app lays them out.
  - `edit` holds `Edit` (`Set`, `Insert`, `Remove`) and `ChangeSet`. Applying
    an `Edit` returns its inverse, which is how undo works. A `ChangeSet`
    groups the edits of one user action and applies all or none:
    `ChangeSet::apply_with` undoes the applied edits when one fails.
    `GameChange` pairs the plain edits of a game-rule edit with their
    inverses.
  - `meta` and `collection` hold `FieldMeta` (label, shape with its storage
    bounds, read-only flag, the game's cap, named values, referenced ID
    tables) and `CollectionMeta`
    (whether rows may be inserted or removed, the key and the fields of a
    new row). Both check an edit before a format writes it.
  - `kinds` holds `Capability` (party, inventory, magic, bestiary), which
    tells the UI which editor sections a save supports, `TableKind`, one
    variant per ID table, and `CapPolicy` ([Edit interfaces](#edit-interfaces)).
  - `error` holds `EditError`, the twelve reasons an edit is refused, and
    `FormatError`, the failure of an operation on files.
  - `files` holds `SaveFiles` (file names and bytes, never a path),
    `FileProbe` (what `detect` sees of a file) and `Detection` (the claim of
    a format).
  - `view` holds `SaveSummary` for the Home screen and `SaveView` for the
    editor, a tree of sections, entities and fields.
  - `format` holds the two traits ([Format traits](#format-traits)).
- **`neoffse-data`** holds the ID tables of a game, keyed by `TableKind`:
  content (items, key items and the five equipment kinds), abilities,
  jobs, commands and monsters, with the name of each entry in English and
  French, plus the game's caps (`MAX_GIL` and the like). It has no maps
  table yet. The TOML files under `data/pr/<game>/` hold the text, and
  `include_str!` embeds it, so a library caller needs no data directory.
  `tables_for(Game)` returns the parsed tables, `None` for a game with no
  data (only FF I has some); `GameTables::parse` parses any six texts.
  Names repeat inside a table, so an ID is the only lookup key. The parser
  checks the file contract, not only the types: IDs start at 1 and rise, an
  ability `level` is 0 to 8 and its `content_id` is not 0, `placeholder`
  appears only as `true`, only on an equipment kind and once per kind, and
  every cap key starts with `MAX_` and holds no negative value. A row that
  breaks a value rule returns `DataError::Rule`, which names the file, the
  row and the rule.
  `tools/pr-tables/` generates the files: a Python script that uses
  UnityPy to read the game's master data and message files from the user's
  install. You run it by hand on a machine where the game is installed
  (`python3 -m prtables --game ff1 --install "<game folder>"`, from
  `tools/pr-tables/`); the build and the tests never run it. The script
  refuses an output folder `<out>/<game>` that lies inside a source folder,
  and a source folder that lies inside `<out>/<game>`.
- **`neoffse-io`** touches the user's files, and knows no save format.
  Six public modules split the work:
  - `error` holds `IoError`, the error type of every operation, and
    `Operation`, the step that failed ([Errors](#errors)).
  - `dirs` holds `DataDir`, which names the folder where NeoFFSE keeps its
    backups. `DataDir::platform` asks the operating system for it
    (`ProjectDirs::data_dir()`). On Linux the folder is
    `$XDG_DATA_HOME/neoffse` when that variable holds an absolute path,
    else `~/.local/share/neoffse`. On Windows it is `%APPDATA%\neoffse\data`:
    `neoffse\data` inside the roaming application data folder.
    A relative result returns `NoDataDir`. `DataDir::at` takes any
    folder, which tests use. The type builds `<data dir>/backups` and
    `<data dir>/backups/<game>` and creates nothing on disk. A game id
    holds only ASCII lowercase letters, digits and `-`, so it stays inside
    the backups folder and two ids cannot collide on a case-insensitive
    file system.
  - `atomic` replaces a file through the verified pipeline of
    [Save safety](#save-safety): `write` takes the bytes and a check to
    run on the temporary file. Every file operation goes through the
    `FileOps` trait, one method per step, so a test makes any single step
    fail. `RealFileOps` is the operating system implementation. The
    provided method `copy_file` copies a file and flushes the copy through
    `sync_file`; backups and the safety backup of a restore copy through
    it, and each failed copy is an `Operation::Copy` error that names the
    source file. Folder operations use `std::fs` and sit outside the
    trait.
  - `backup` has `create`, `list` and `restore`. A backup is a folder of
    copied files with a `backup.json` manifest (`Manifest`); `list`
    returns a game's readable backups, newest first, and reports the
    folders it cannot read; `create` keeps the 20 newest backups of a
    game. [Save safety](#save-safety) gives the layout and the rules.
  - `discovery` finds the save folders of Steam games. The caller fills an
    `Environment` (`Environment::platform` reads the real one) and passes
    `SteamGame` descriptors: a game id, a Steam app ID and a folder name
    under `My Games`. `discover` returns a `Discovery`: `folders`, one
    `SaveFolder` per `My Games/<folder>/Steam/<SteamID64>` found, with its
    `Origin`, and `skipped`, the places it could not read as `IoError`s. A
    path that does not exist is no error. `discover` returns each path
    once: when two descriptors name one folder, the first descriptor gives
    the id. The search never writes.
  - `process` answers one question: which of these executables have a
    live process? `running` takes the names and returns the matching ones.
    The scan is best effort: it skips threads, never fails, and a process
    the system hides stays invisible, so an empty result proves nothing.
    `Key` picks the field to compare (`Key::Name` on Windows,
    `Key::CommandLine` elsewhere); [Save safety](#save-safety) gives the
    reason.

  Discovery on Linux collects Steam roots (native, Flatpak, Snap).
  `<data_home>/Steam` and `~/.local/share/Steam` are both probed, since
  the first is the second unless `XDG_DATA_HOME` moves it, and the
  candidate list drops an exact duplicate before it canonicalizes a
  path. The Steam root is a library, even when its
  `libraryfolders.vdf` omits it, and that file lists the others. Discovery
  canonicalizes every library first, then probes it for
  `compatdata/<app ID>`, because a Proton prefix can sit outside the
  game's library. A library that two roots list is probed once, and the
  first root owns it (`Origin::SteamLibrary::root`). A library path that
  is not absolute is skipped and reported in `skipped` as
  `LibraryListInvalid`: it would otherwise resolve against the working
  folder of the process. The public module
  `neoffse_io::discovery::library_list` exposes `library_paths`, a pure
  parse of the list text that touches no disk.

  On Windows discovery probes four candidates in this order, and drops a
  candidate that resolves to a folder it already probed: the
  `Environment::documents` field (the Documents Known Folder),
  `%USERPROFILE%\Documents`, `%USERPROFILE%\OneDrive\Documents` and
  `%OneDrive%\Documents`. A result keeps the spelling of the first
  candidate that named its folder. After a OneDrive Known Folder Move, a
  stale local `Documents` folder that still holds a save is a different
  folder from the synced one, so discovery returns two `SaveFolder`s and
  the caller chooses
  ([Save folder](formats/pixel-remaster.md#save-folder) lists the Go
  editor's candidates).

  `discover` runs both searches on any system, and each search runs only
  when the `Environment` fields it needs are set. A relative path in an
  `Environment` field counts as absent and is not reported, and a relative
  or missing `home` switches off the Linux search. The rules behind
  discovery are in
  [Steam roots and libraries](formats/pixel-remaster.md#steam-roots-and-libraries).
  Each format crate declares where its saves sit by publishing plain
  data, and the caller turns it into `SteamGame` descriptors:
  `neoffse-format-pr` publishes `PrGame` rows in `games::GAMES` and has no
  dependency on `neoffse-io`.
- **`neoffse-format-pr`** reads and writes FF I to VI PR saves. The codec
  splits off the BOM and the trailing whitespace, decodes base64,
  decrypts with Rijndael-256, inflates the raw deflate stream and returns
  the JSON text. The decrypted stream keeps its zero padding, because the
  inflater stops at the final block and so ignores it, and the codec
  records the BOM and the exact trailing bytes so that `encode` writes
  them back; a trailer longer than `MAX_TRAILER_LEN` (64 bytes, against
  the 2 the game writes) fails the codec, and `encode` refuses a JSON text
  above `MAX_PAYLOAD_LEN` (16 MiB), the cap `decode` applies. The lossless
  document reads and edits that text by path, and inserts or removes the
  elements of an array. The `games` module holds `GAMES`, the six
  `PrGame` entries of FF I to VI: game id (`ff1-pr` to `ff6-pr`), Steam
  app ID, folder under `My Games` and executable name. A caller copies
  the first three fields into `neoffse-io`'s `SteamGame` and passes the
  executable names to `process::running`. A test pins the table to the
  [Games](formats/pixel-remaster.md#games) table of the format document.
  `games::identify` tells the game of a slot document by the entry count
  of its vehicle list and a few marker keys ([Telling the games
  apart](formats/pixel-remaster.md#telling-the-games-apart)); every PR
  format applies it when it opens a file, and `Ff1Format` does so today. The `slots` module holds the 22
  slot file names the six games share and `detect`, which claims a file by
  its name: `Likely` when the head also looks like base64, `Possible`
  otherwise, never `Certain`, because the head names no game.
  The `ff1` module maps an FF I slot onto the domain of `neoffse-core`:
  - `Ff1Model` wraps a `Document` and the FF I tables of `neoffse-data`.
    It reads and writes fields by `FieldId`, never by position, with the
    metadata of each field (`field_meta`, `collection_meta`). It offers the
    two [edit interfaces](#edit-interfaces), `apply` and `apply_game`,
    plus `row_locked` and `field_locked`. It also builds the `SaveSummary`,
    whose party follows `corpsList`, and the `SaveView`, which lists every
    character in the order of `ownedCharacterList`. A key the model does
    not map stays as read.
  - `Ff1Format` implements `SaveFormat`: `open` takes exactly one file and
    decodes it. It maps the vehicle count to a game and refuses a count of
    another game, then refuses the marker keys of another game
    ([Telling the games
    apart](formats/pixel-remaster.md#telling-the-games-apart)). It then
    checks the shape: the character list and the two item lists exist, each
    row key is an unsigned 32-bit integer, and no `id` or `contentId`
    repeats inside a list, since the model finds a row by its key.
    `OpenError` says why a file is no slot of the game: wrong file count,
    not a slot, or a slot of another game.
  - `Ff1Document` implements `SaveDocument` over the model and reports the
    capabilities `Party` and `Inventory`; the `Magic` section arrives with
    M4 ([Roadmap](roadmap.md#m4-app-ff-i-end-to-end)). It keeps the BOM, the
    trailer and the file name of the opened file, so `encode` writes the file
    back under its own name. `verify` decodes the bytes and compares
    the JSON text and the container with the in-memory document.
  - `names` holds the field vocabulary: the section names `gil`,
    `character`, `inventory` and `key_items`, and the 39 field ids of a
    character.

  [`docs/formats/pixel-remaster.md`](formats/pixel-remaster.md) holds
  every byte-level rule, with the codec layers under
  [Container encoding](formats/pixel-remaster.md#container-encoding).
- **`neoffse`** (M4) re-exports the crates above so a caller depends on one
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

A format registers one `SaveFormat` per game. Opening a save yields a
`SaveDocument`, which owns the original data and every edit applied to it.
Both traits live in `neoffse_core::format`; the document trait is
dyn-compatible and `Send`, so the app moves a document to a worker thread.

```rust
pub trait SaveFormat: Send + Sync {
    fn game(&self) -> Game;
    fn detect(&self, probe: &FileProbe<'_>) -> Option<Detection>;
    fn open(&self, files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError>;
}

pub trait SaveDocument: Send {
    fn game(&self) -> Game;
    fn capabilities(&self) -> &[Capability];
    fn summary(&mut self) -> Result<SaveSummary, FormatError>;
    fn view(&mut self) -> Result<SaveView, FormatError>;
    fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError>; // returns the inverse
    fn apply_all(&mut self, changes: &ChangeSet, policy: CapPolicy) -> Result<ChangeSet, EditError>; // provided
    fn apply_game(&mut self, edit: &Edit) -> Result<GameChange, EditError>;        // provided
    fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError>;              // provided
    fn field_locked(&self, field: &FieldId) -> Result<bool, EditError>;            // provided
    fn encode(&self) -> Result<SaveFiles, FormatError>;
    fn verify(&self, files: &SaveFiles) -> Result<(), FormatError>;
}
```

- **Files, never paths.** `SaveFiles` carries file names and bytes, so a
  format cannot reach the disk or choose where a file lands. A save that
  spans several files (a memory card, a PC game that splits a slot) fits the
  same type. `Debug` shows the names and the byte counts only.
- **Detection.** `detect` is pure. It reads a `FileProbe` (the name, the
  size and at most 256 bytes of the head) and returns a `Detection`, or
  `None`. `Confidence` runs `Possible` (the name fits), `Likely` (the name
  and a cheap content check fit), `Certain` (the first bytes prove it), so
  the registry picks the best claim when two formats could claim a file. The
  six PR formats claim the same file names, so the facade (M4) breaks the
  tie with the game of the save folder. A format never claims a leftover
  `.neoffse-*.tmp` file of the atomic write, and the registry of the facade
  (M4) skips such names before it calls `detect`.
- **Open.** `open` copies what it needs from the files, so the document
  keeps no borrow. It refuses a file that does not fit its game with a
  `FormatError::Decode`, never a panic.
- **Views.** `summary` feeds the Home screen: slot, play time, place and
  party. `view` returns the editable content as a snapshot, one section per
  capability, each field with its `FieldMeta` and current value, each
  collection with its `CollectionMeta`. Both take `&mut self` because a
  format may parse lazily; neither changes a value.
- **Encode and verify.** `encode` returns the files to write under the
  names `open` received. `verify` decodes such files from their bytes alone
  and compares the result with the whole in-memory document, including the
  data the view omits. The facade (M4) calls it from the hook of
  `neoffse_io::atomic::write` ([Save safety](#save-safety), step 6).

### Edit interfaces

A document edits through two interfaces over one `Edit` type. The UI
picks the interface; the library keeps no mode.

An `Edit` is `Set` (a field takes a value), `Insert` (a row joins a
collection at a position) or `Remove` (a row leaves it). Every `apply`
returns the inverse edit, and an inverse of a `Remove` restores the row at
its old index with its values. Redo is the original edit applied again.

- **Raw: `apply(edit, policy)`.** The edit goes through the storage bounds
  of the field, and through the game's cap and the ID tables when the policy
  is `CapPolicy::Enforce`. `CapPolicy::StorageOnly` accepts any value the
  save can hold, which is the app's expert mode. The raw interface holds no
  game rule: it writes a bare equipment id, a count below the number of
  wearers, or a placeholder row, if the value fits the bounds. An inverse,
  a rollback and a redo always run under `StorageOnly`, because they restore
  a value the document already held and the user may have set it above the
  cap.
- **Game: `apply_game(edit)`.** The edit changes the save the way the game
  would, so the result keeps the invariants the game keeps. Caps and ID tables are always
  enforced, and the game rules of the format hold. FF I equips an item by
  setting `character.<id>.equipment.<slot>`: the call expands it into the
  slot's `contentId`, an inventory row that holds the item for every wearer,
  and the slot `count` copied from the inventory. A count below the number of
  wearers returns `OutOfRange` with that number as `min`, and a key item
  inserted with a `count` other than 1 returns `OutOfRange` with `min` and
  `max` at 1: `OutOfRange` names the range the edit broke, which under this
  interface can be a game floor. The call refuses
  with `ReadOnly` to remove a worn item, to edit a placeholder row, and to
  set the `count` of an equipment slot or of a key item. It returns a `GameChange`: the
  plain edits it applied (`forward`) and their inverses (`inverse`). Both
  sets replay through `apply_all` under `StorageOnly`, which holds no game
  rule, so neither can fail on one. The default body of the trait method
  applies one edit under `Enforce` and holds no other rule.
- **Locks.** `row_locked(row)` and `field_locked(field)` tell the UI which
  controls the game interface refuses whatever the value, so it greys them
  before the click. They agree with `apply_game`: a refusal that depends on
  the value is an error, not a lock. A row that the model cannot restore
  byte for byte after a removal is locked too, for example a row whose stored
  `count` lies outside the storage bound: `Remove` refuses it on both
  interfaces.
- **All or none.** `apply_all` applies a `ChangeSet` and, when an edit
  fails, undoes the applied edits newest first, so the document holds all the
  changes or none. `Ff1Document` also snapshots the document for a set of
  two or more edits: a rollback rewrites the levels it touched in compact
  form, and the snapshot brings the original text back byte for byte.
  When an undo fails too, the result is `EditError::RollbackFailed` and the
  caller reloads the document.

### Technology stack

The workspace uses the crates below; each `Cargo.toml` pins the version.
A crate tagged with a milestone enters the workspace with it.

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
| File dialogs (M4) | `rfd` | native dialogs, XDG portal on Linux (Flatpak-safe) |
| JSON | `serde`, `serde_json` | `neoffse-format-pr` uses `serde` with `std` only and `serde_json` with `preserve_order` (key order), `arbitrary_precision` (a number keeps its written text) and `raw_value` (an untouched value keeps its bytes); they sit under a lossless document layer. `neoffse-io` uses `serde` with `std` and `derive` and `serde_json` with its default features, for `Manifest` |
| Deflate | `flate2` (`rust_backend`) | raw deflate, no zlib header, pure Rust |
| Base64 | `base64` (`std` only) | standard alphabet, `=` padding |
| ID table text | `toml` (`parse`, `serde`, `std`), `serde` (`std`, `derive`) | `neoffse-data` parses the embedded `data/pr/<game>/*.toml`; each raw row refuses unknown keys, so a column the export tool adds fails the tests until the parser handles it |
| PR cipher | hand-ported Rijndael-256 | 32-byte block, 32-byte key, CBC, fixed key and IV, zero padding; AES fixes the block at 16 bytes, so AES crates cannot decrypt it. The port computes its S-box and lookup tables by `const fn` and expands the key schedule in `Rijndael256::new` |
| ID table export | Python 3.11 or later and `UnityPy` in `tools/pr-tables/` | reads the master data and message files of the user's install; `requirements.txt` pins every package. Only a contributor who regenerates the tables needs Python |
| Cipher test vectors | Go program in `tools/cipher-vectors/` | runs KiameV's ffpr-save-cypher (version pinned in its `go.mod`). Only a contributor who regenerates the vectors needs Go |
| Binary formats (after M5) | `binrw` | for the console and older PC formats after the PR |
| Atomic write | `tempfile` | temp file in the target folder, then rename |
| Data dir | `directories` | `ProjectDirs::data_dir()` locates the backups ([Save safety](#save-safety)) |
| Steam library list | `keyvalues-parser` | reads `libraryfolders.vdf` |
| Running game | `sysinfo` | process scan before a write. Built with `default-features = false` and the `system` feature. Windows matches `name()` to the executable name; Linux matches the last component of `cmd()[0]`, because the kernel cuts the process name to 15 bytes ([Save safety](#save-safety)). Its minimum Rust version, 1.95, sets the `rust-version` of the workspace |
| Errors | `thiserror` in libraries, `anyhow` in the app (M4) | [Errors](#errors) |
| Logging (M4) | `tracing` | |
| Tests | `proptest` (`std` only); `insta` (M4) | property tests on codecs and on the document; snapshots |

Cargo unifies the features of a dependency across a workspace build, and
the app's dependency tree turns on features that `neoffse-format-pr` does
not request. For that reason CI runs the tests of `neoffse-format-pr` alone.

## Errors

A library crate returns errors; it does not panic on anything read from
a file.

- One `thiserror` enum per failure domain of a library crate.
  `neoffse-format-pr` has `CodecError` (the container steps),
  `DocumentError` (reads and edits of the JSON document) and `OpenError`
  (why a file is no slot of a game: wrong file count, not a slot, a slot of
  another game). `neoffse-core` has `FormatError` for the failure of a
  format operation (an invalid or repeated file name, a missing file, a
  decode, encode or verify failure); a format crate converts its own errors
  into it and keeps them as the source. `neoffse-core` also has `EditError`,
  the reasons an edit is refused. `neoffse-data` has `DataError` (a
  text that is no valid TOML, an unknown kind, an ID out of order, or a row
  that breaks a rule of the file contract, `Rule`). `neoffse-io` has `IoError`: a failed
  file operation with its step and path (`Io`), a data folder with no
  location (`NoDataDir`), an invalid game id (`InvalidGameId`), a failed
  verification (`VerificationFailed`), an invalid backup manifest
  (`ManifestInvalid`), an invalid Steam library list
  (`LibraryListInvalid`) and a restore that stopped partway
  (`RestoreFailed`). The app (M4) uses `anyhow` and adds context at the
  boundary.
- Every error enum is `#[non_exhaustive]`, so a `match` needs a wildcard
  arm and a later version may add a variant. `IoError` and its
  `Operation` enum follow the rule.
- `Display` never repeats the source error, so an `anyhow` chain prints
  each cause once.
- No panic path on input data. The workspace lints deny `unwrap_used`
  and `expect_used`; a test root allows both, since a test asserts by
  panicking.
- An error carries the position when the code knows it, and its variant
  names the step that failed. `CodecError::Base64` holds the byte offset
  in the file, `CodecError::Inflate` the offset in the decrypted stream.
  Every `DocumentError` variant except `TooDeep` (which holds the limit)
  carries a rendered path, for example
  `userData.ownedCharacterList.target[2].parameter`: keys join with dots,
  an index shows in brackets, and the step into a string's document shows
  nothing. `InvalidJson` also keeps the parser's error, whose line and
  column count from the first byte the caller passed.
- `CodecError::PayloadTooLarge` holds in `limit` the cap that fired, the
  payload cap or the base64 text cap. `decode` raises it for an inflated
  payload above `MAX_PAYLOAD_LEN` and `encode` for a JSON text above it. `CodecError::TrailerTooLong` holds
  the trailer length and its limit, `MAX_TRAILER_LEN` (64 bytes); `decode`
  and `seal` both refuse a longer trailer.
- `IoError::RestoreFailed` lists the files `written` and the files
  `remaining`, carries the `safety` backup that undoes the restore, and
  keeps the error of the file that failed as its source. `source()` returns
  the `Box<IoError>` itself, so a caller reads the cause with
  `error.source().and_then(|source| source.downcast_ref::<Box<IoError>>())`.
- `EditError::RollbackFailed` means a change set failed and its undo failed
  too: the document is in an unknown state and the caller reloads it. Its
  `source()` returns the `Box<EditError>` itself, so a caller reads the
  error that stopped the change set with `downcast_ref::<Box<EditError>>()`;
  `downcast_ref::<EditError>()` finds nothing. The error of the undo sits
  outside the source chain, in the `rollback` field.
- **Errors hold personal paths.** The `Display` of `IoError::Io`,
  `VerificationFailed`, `ManifestInvalid` and `LibraryListInvalid` names a
  path under the user's home, and the `source` of `LibraryListInvalid` can
  quote a line of `libraryfolders.vdf`. The same holds for the backup and
  restore errors, for each entry of `Discovery::skipped`, and for the
  `Debug` output of `Discovery`, `SaveFolder` (which also holds a
  SteamID64), `ProcessInfo` (the first command-line argument of a process)
  and `Manifest` (`sourceDir`). The `Debug` output of the following types
  holds paths under the home too: `IoError` (its variants that carry a path),
  `Backup` (`dir` and `manifest.source_dir`), `Listing`, `Restored` (its
  `safety` backup), `Request` (`source_dir`), `Environment` (every
  field), `Origin` and `DataDir` (the root). The data folder is a path
  under the home, so an error that names a backup folder holds it. No
  function of the crate logs or prints any of them, and a caller must not
  print them raw: log the variant and the `Operation`, or the count of
  skipped places.
- **Save content in `Debug` output.** These types print character names,
  the place, field values, the head of a file or the JSON text of a slot, so
  a caller must not log them: `SaveSummary`, `SaveView`, `Section`,
  `Entity`, `FieldView`, `Value`, `Edit`, `ChangeSet`, `GameChange`,
  `FileProbe`, `Document`, `Decoded` and `Ff1Model`. By field type, so do
  the `Scalar` of the document and the `Inflated` and `Framing` of the codec.
  The rustdoc of each type says "`Debug` prints save content: never log it."
  `SaveFiles` prints file names and byte counts only, and `Ff1Document`
  prints its type and file name. `FormatError` and `OpenError` hold file
  names, counts and document paths, never a path on disk.
- A refused edit returns an error with the field and the reason, and the
  document stays as it was before the call, except after
  `EditError::RollbackFailed`, where the caller reloads the document.
  `Document::set` behaves so with `DocumentError`. `EditError::OutOfRange`
  carries the range the edit broke: the storage bound or, under
  `apply_game`, a game floor such as the number of wearers.
  `EditError::AboveCap` carries the game cap, which the UI reads to offer
  expert mode. `EditError::Format` and `FormatError::Decode` keep the
  document or codec error as their source.

## Test policy

`cargo test --workspace` passes on a fresh clone without any game file.

- **Tests live in `tests/`.** No source file holds test code, in any
  crate. A crate's tests sit in its `tests/` directory as integration
  tests and use the public API. An internal stage that needs tests, such
  as a codec step or the cipher, is exposed in a documented public
  module. Helpers that several test files share sit in
  `tests/common/mod.rs`, which Cargo does not build as a test target.
- **Round trip on every fixture.** Decode, encode, decode again: the two
  decoded documents and their containers must match. The game's deflate
  encoder and `flate2` choose different streams for the same input, so a
  PR save never re-encodes to the game's bytes, and PR round trips
  compare the inflated JSON text byte for byte. The game accepts a
  deflate stream that differs from its own: FF I loads a bestiary file
  the Go editor re-encoded. On real saves, the round trip also compares
  the BOM and the trailing whitespace of the re-encoded file with the
  original file.
- **Edit tests.** An edit, then encode and decode, yields the intended
  change and no other difference in the JSON text.
- **FF I exit proof.** The proof runner of `neoffse-format-pr`
  (`tests/common/edit_proof.rs`) drives an edit through
  `Box<dyn SaveDocument>`, encodes, decodes with the codec and a JSON tree
  of its own, and compares the tree with the one before the edit. The only
  differences allowed are the expected ones: the edited tokens and, for a
  slot stored with spaces, the compact rewrite of the levels the edit
  crosses. The runner checks that each rewritten level is compact. It then
  undoes the edit, on the edited document and on a reopened one, and applies
  it again. After an undo the original text returns; on a slot stored with
  spaces it returns up to the compact rewrite of the levels the edit
  crossed. An edit matrix runs on the synthetic slot
  `fixtures/pr/ff1-slot.json` and on every private FF I slot: gil, level,
  experience, HP, max HP gain, power, luck, magic, job, name, spell
  charges, max charges gain, item count, row insert and remove, and equip
  and unequip in each of the five equipment slots. Every case runs through
  `apply_game`; the cases other than the equips also run through `apply`
  under `Enforce`. A case whose ingredient a save lacks skips. The runner
  fails on a change it did not expect and on a change it expected and did
  not find, and the `prove_fails_on_*` tests pin those failures. Guard
  cases check the refusals: a refused edit and a set to the current value
  leave the file as it was, and each container comes back unchanged. On
  private saves a failure names the slot label and a field path, never a
  value.
- **Domain contract.** `neoffse-core` tests the edit contract with
  hand-built documents and `proptest`: a refused edit changes nothing, an
  inverse restores the document, and a `ChangeSet` applies all or none.
  `neoffse-data` tests the embedded FF I tables against the facts of
  [Game master data](formats/pixel-remaster.md#game-master-data).
- **Properties.** `proptest` drives each codec stage (framing, base64,
  the cipher with its padding, deflate), the whole pipeline and the
  document with generated input. Decode inverts encode, an integer edit
  changes only its own digits, and no input makes a stage panic. A failing
  case lands in `tests/*.proptest-regressions`; commit that file with the
  fix that makes the case pass.
- **Cipher vectors.** A Go program in `tools/cipher-vectors/` runs
  KiameV's ffpr-save-cypher and writes Rijndael-256 vectors (single
  blocks, CBC encryption, CBC decryption with the length the Go `Decrypt`
  returns) to `crates/neoffse-format-pr/tests/data/rijndael256-vectors.txt`.
  The repository commits that file and the Rust port must reproduce every
  vector. The output depends only on the pinned library version and a
  fixed seed. Before it writes, the generator checks that the library's
  CBC IV equals the IV documented in
  [Cipher](formats/pixel-remaster.md#cipher). To regenerate the file, run,
  with Go installed:
  `cd tools/cipher-vectors && go run . -o ../../crates/neoffse-format-pr/tests/data/rijndael256-vectors.txt`.
- **Fixtures.** Synthetic fixtures are committed under `fixtures/`, and
  `neoffse-testkit` locates and reads them. The six container fixtures
  of `fixtures/pr/` come from one test, which fails when a committed file
  differs from what the generator writes, then decodes, encodes and
  decodes each file again. A compression update that changes the stream
  fails the first check; regenerate the files with
  `NEOFFSE_WRITE_FIXTURES=1 cargo test -p neoffse-format-pr --test container_fixtures write_fixtures`.
  The FF I slot `fixtures/pr/ff1-slot.json` holds invented values that obey
  the game's rules (equipped items sit in the inventory, slot counts copy
  the inventory count, the five placeholder rows are present). A test fails
  when the file differs from its generator; regenerate it with
  `NEOFFSE_WRITE_FIXTURES=1 cargo test -p neoffse-format-pr --test ff1_fixture write_fixture`.
- **Real saves.** They are personal data and stay out of the repository.
  A test asks the testkit for the saves of one game, for example
  `neoffse_testkit::private_saves("ff1")`. The testkit reads the folder
  named by `NEOFFSE_SAVES`, `fixtures/private/` by default. A relative
  value resolves from the workspace root, and `~` is not expanded. When
  the folder holds nothing for that game, the call returns `None` after
  `neoffse_testkit::skip` writes the reason on stderr; the test returns
  early and passes. A wrong `NEOFFSE_SAVES` path skips every real-save
  test, and the run stays green; `cargo test -- --nocapture` shows the
  reasons. The testkit tests its skip path through a child process,
  because editing the environment of a running test needs `unsafe` in
  edition 2024 and the workspace denies `unsafe_code`. The PR
  tests ignore `steam_autocloud.vdf`, the marker Steam writes beside
  saves, and treat a game folder that holds only that file as absent.
  Real-save tests only read.
- **Real discovery.** `crates/neoffse-io/tests/real_discovery.rs` holds
  one test marked `#[ignore]`, because it reads the Steam install of the
  machine it runs on. For each of the six PR games it runs `discover` with
  `Environment::platform()` and checks that discovery finds a save folder
  and that every folder holds a file `neoffse_format_pr::container::decode`
  accepts. It reads and never writes. Its output and its failure messages
  carry game ids and counts only, never a path, a SteamID64 or a file
  name. On Linux, run
  `cargo test -p neoffse-io --test real_discovery -- --ignored --nocapture`.
  For Windows, build the test executable on Linux with
  `cargo test -p neoffse-io --test real_discovery --target x86_64-pc-windows-gnu --no-run`
  (this needs `mingw-w64` and the `x86_64-pc-windows-gnu` Rust target),
  copy `target/x86_64-pc-windows-gnu/debug/deps/real_discovery-<hash>.exe`
  to the Windows machine and run it there with `--ignored --nocapture`.
  `neoffse-format-pr` is a dev-dependency of `neoffse-io` for this test
  only, so the I/O crate stays free of any format.
- **Test profile.** The root `Cargo.toml` sets `opt-level = 2` in
  `[profile.test]`, so `cargo test` builds optimized code. The cipher and
  the property tests are CPU bound, and the largest codec test,
  `max_text_len_admits_the_worst_case_payload`, encrypts and decrypts
  about 16 MiB.
- **Snapshots (M4).** `insta` will pin change summaries and view model
  state.
- **UI.** View models (M4) carry the UI logic and run in plain Rust
  tests. The Slint layer runs on Slint's testing backend, through
  `i-slint-backend-testing`: its tests call `init_no_event_loop`, so
  they create no window and no event loop. No test opens a window on the
  developer's desktop.
- **Platforms.** CI (`.github/workflows/ci.yml`) runs on each push to
  `develop` and to a milestone branch (`v*.*.*`), and on each pull
  request into `develop`. The `checks` job runs on Linux and on Windows:
  `cargo fmt --all --check`, `cargo fetch --locked` (fails when
  `Cargo.lock` no longer matches the manifests), `scripts/clippy.sh`,
  then `cargo test --workspace --locked`. On Linux, it also runs
  `cargo test -p neoffse-format-pr --locked`, because feature unification
  across the workspace can hide a missing feature of that crate, and
  `cargo doc --workspace --no-deps --locked` with
  `RUSTDOCFLAGS=-D warnings`, so a broken intra-doc link fails the build.
  The `msrv` job runs `cargo check --workspace --all-targets --locked`
  on Linux with Rust 1.95.0, the `rust-version` of the workspace. CI
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
- A Rust source or test file of the workspace holds 300 lines at most.
  Split it into modules before it gets there. The Go program under
  `tools/` is outside the rule.
- The lint set lives in the root `Cargo.toml` under `[workspace.lints]`:
  clippy `pedantic` and `cargo` as groups, `allow_attributes_without_reason`,
  `unwrap_used`, `expect_used`, `print_stdout` and `print_stderr` by name,
  and rustc's `missing_docs`, `missing_debug_implementations`,
  `unreachable_pub` and `unsafe_code`, all deny. `scripts/clippy.sh` runs
  clippy with `-D warnings` over every target. An exception lives in the
  code as `#[allow(..., reason = "...")]`, and every `allow` carries its
  reason; a script flag never serves as an exception.
  One exception sits in a configuration file, not in the code:
  `clippy.toml` sets `allowed-duplicate-crates = ["syn"]` for
  `clippy::multiple_crate_versions`. `syn` is a proc-macro build
  dependency and never reaches a binary. The `windows` 0.62 crate that
  `sysinfo` pulls on Windows builds its macros on `syn` 2, while
  `serde_derive` and `thiserror-impl` build on `syn` 3.
- Run `cargo fmt` with the committed `rustfmt.toml` (`max_width = 160`)
  before every commit.

## UI architecture

The screens below land with M4; M0 ships the window, the top-level menu
and the About dialog. The `.slint` files draw; Rust view models decide.
A view model turns
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
- **Default and expert mode.** In the default mode the view model sends a
  new edit through `apply_game` and keeps the returned `GameChange` for
  undo and redo; it greys a control that `field_locked` or `row_locked`
  reports. In expert mode it sends the edit through `apply` with
  `CapPolicy::StorageOnly`, and an `AboveCap` error from the default mode
  is the cue to offer the switch ([Edit interfaces](#edit-interfaces)).
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
  - *Advanced*: the raw document tree, behind a warning. No milestone
    schedules it yet.
  - *About*: version, licenses, and the `AboutSlint` widget, reached
    from the top-level menu.
- **Assets.** The app asks an `AssetProvider` for portraits and icons;
  [Visual assets](#visual-assets) describes the providers.

## Visual assets

The app (M4) picks each image from the best of three providers, and the
repository never holds a graphic or audio asset extracted from a game. One
`AssetProvider` trait in the app layer hides the choice, so no library
crate depends on it. The providers, in order of preference:

1. **Local extraction.** The app extracts portraits from the user's own
   game installation into the user cache directory
   (`~/.cache/neoffse/assets/<game>/` on Linux). NeoFFSE never commits
   or redistributes these files. The provider will sit behind the
   `local-assets` cargo feature of `neoffse-app`, off by default, so
   public builds leave it out. The provider needs the game install
   folder (`steamapps/common/<game folder>`), which `neoffse-io`
   discovery does not locate: it finds save folders only.
   `NEOFFSE_GAME_DIR` will override the install lookup. The
   Pixel Remaster games store their art in Unity asset bundles, and the
   provider needs research on that format before any code. No milestone
   schedules it.
2. **Bundled free art.** Files in `assets/` with a known author and a
   free license, attributed per file in REUSE style (`REUSE.toml`), the
   way ff7tk does for Black Chocobo. The folder holds no such file yet.
3. **Silhouettes (M4).** Original generic images in `assets/`. They
   replace any missing portrait, and public builds show no other images
   for now.

A user who owns the game sees real portraits on their own machine, and a
public build redistributes nothing the project has no right to ship.

## Save safety

A save reaches disk through one pipeline in the `neoffse` facade (M4), and
any failure before the final rename leaves the original file as it was.
`neoffse-io` provides the backup, write, rename and process steps (M2),
and the facade (M4) joins them to the format's encoder and decoder.

1. The facade checks for a running game process through the
   [`process`](../crates/neoffse-io/src/process.rs) module of
   `neoffse-io`, and the app warns when it finds one. The user may cancel
   or continue. A PR game runs under its Windows executable name
   (`FINAL FANTASY II.exe`). On Windows the check compares the process
   name. On Linux the game runs under Wine, where the kernel cuts the
   process name to 15 bytes: FF II, FF III and FF IV all read
   `FINAL FANTASY I`. The loader that `/proc/<pid>/exe` names is the same
   for every Wine process, so it cannot name the game either. Under
   Proton, Wine rewrites the first command-line argument to the Windows
   path of the game executable, for example
   `<drive>:\<path>\FINAL FANTASY II PR\FINAL FANTASY II.exe`, and the
   arguments after it are empty (measured on FF II); `sysinfo` drops the
   empty ones. The check takes the last component of that first argument
   (`cmd()[0]`), split on `/` and on `\`, and compares it with the
   executable name, ASCII case folded. It skips threads. The scan is
   best effort: a process the system hides stays invisible, so the check
   can warn but never prove that the game is closed.
2. The user reviews the change summary and confirms.
3. `neoffse-io` copies the current files into a new backup folder and
   writes its `backup.json`.
4. The format encodes the document. The PR encoder refuses a JSON text
   above `MAX_PAYLOAD_LEN` (16 MiB), the cap the decoder applies, so the
   pipeline never writes a file that step 6 or a later load would refuse.
5. `neoffse-io` writes the bytes to a temporary file in the save's own
   folder, so the rename stays on one filesystem, and flushes it to
   disk.
6. The format decodes the temporary file, and the facade compares the
   result with the in-memory document (`SaveDocument::verify`, which
   covers the whole document, not only the view). The facade hands this check to
   `neoffse_io::atomic::write` as a hook. On a mismatch, or on any other
   failure before the rename, a guard in `neoffse-io` deletes the
   temporary file, also when the hook panics, and the call returns an
   error.
7. `neoffse-io` renames the temporary file over the original.

In the facade (M4), step 3 creates the backup before step 4 encodes, so
an encode that fails leaves a backup that counts against the 20 of its
game.

**Backup layout.**

```
<data dir>/backups/<game>/<YYYYMMDD-HHMMSS>[-NN]/
├── backup.json
└── <the save files, under their original names>
```

`<data dir>` is the platform data folder that `neoffse_io::dirs::DataDir`
names: `$XDG_DATA_HOME/neoffse` (default `~/.local/share/neoffse`) on
Linux, `%APPDATA%\neoffse\data` on Windows. Backups hold user data, so
they sit in the data folder. `<game>` is a game id of lowercase ASCII
letters, digits and `-`, such as `ff1-pr`.

The folder name is the UTC time of the backup's `Moment`, an instant with
an offset from UTC (`Moment::now` reads the system clock in UTC), and it
has no `Z`. `created` in the manifest shows the local time with that
offset, and `Z` for UTC. A `Moment` holds a UTC time and a local time that
both fall in the years 1970 to 9999, so the year has four digits. `-NN`,
from `-01` to `-99`, separates backups taken in the same second. A new
backup takes the number after the highest one on disk, so no name is
reused inside a second, even after retention deleted its folder; the 101st
backup in one second fails with an `AlreadyExists` error. A backup holds
only the files its request names, which is the edited slot and not the
whole save folder. The manifest (illustrative values):

```json
{
  "version": 1,
  "game": "ff1-pr",
  "sourceDir": "/home/user/.local/share/Steam/steamapps/compatdata/<appid>/pfx/drive_c/users/steamuser/Documents/My Games/...",
  "created": "2026-10-18T20:14:03Z",
  "files": [
    "<slot file>"
  ],
  "saveType": "slot",
  "reason": "before-save"
}
```

- `neoffse-io` keeps the 20 newest readable backups per game and deletes
  older ones after a backup succeeds. The count covers the whole game,
  not each slot, so edits to one slot can push out the last backup of
  another. A folder with a backup name but no readable `backup.json` is
  reported by `list` and never counted or deleted.
- A restore reads the manifest and every backup file first, so a missing
  or unreadable file stops it before any write. It then backs up the
  files of the backup that exist in the target folder (`"reason":
  "before-restore"`) and writes the backup's files one by one through the
  atomic write of steps 5 to 7. No format takes part: the check of step 6
  compares the bytes of the temporary file with the bytes of the backup,
  and nothing decodes or encodes. The user can undo a restore by
  restoring the new backup. A file that the target folder did not hold is
  created and listed in `Restored::created`; undoing the restore does not
  delete it.
- A restore that fails while it writes returns `IoError::RestoreFailed`.
  The files already written keep the backup's version, and no rollback
  runs, because it would write to the disk that refused the last write.
  The error carries the safety backup as the way back.
- The backup being restored stays out of the retention count of the
  restore's safety backup, so a game holds at most 21 readable backups
  after a restore, until the next `create` prunes back to 20. A game that
  held fewer than 20 backups gains at most the safety backup.
- `restore` writes into any existing folder it receives and does not
  compare it with the `sourceDir` of the manifest, a capture of the
  folder at backup time that a library move or a reinstall can make
  stale. The facade (M4) must derive `target_dir` from a fresh
  discovery or from the manifest.
- **Steam Cloud.** Steam syncs PR saves to its cloud. After an edit,
  Steam may report a conflict at the next launch; the user must keep the
  local files, or Steam replaces the edit with the cloud copy. The app
  (M4) shows this note after a save.

## Layers and milestone boundaries

Each milestone closes one or more crates, and an audit reviews each
closed crate before the next milestone opens. The
[roadmap](roadmap.md) holds the scope and exit criteria.

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

- **macOS.** CI covers Linux and Windows. macOS builds need a third CI
  runner and a tester on that system.
- **crates.io.** Publishing `neoffse` and its internal crates on
  crates.io remains open. Every crate sets `publish = false` until that
  choice, and the manifests carry the metadata `clippy::cargo` requires,
  so either outcome costs no rework. `neoffse-data` embeds `data/pr/`
  through a path that leaves its crate folder, so a publication first
  copies the tables into the crate.
