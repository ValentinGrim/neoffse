//! What the six Pixel Remaster games share: the section names every slot
//! uses, the document paths of its three collections, and the reads that
//! find a row or a dictionary entry by key
//! (`docs/formats/pixel-remaster.md#container-shapes`,
//! `docs/formats/pixel-remaster.md#documents-inside-strings`).
//!
//! The layer also holds the character-field machinery: the field bindings,
//! the reads, the raw and game edits, the summary of a slot and the party
//! view. A game supplies its `Part` enum of character fields, its tables, its
//! rules and the lists whose keys must not repeat, and calls this layer for
//! the rest.
//!
//! A game plugs into [`SlotFormat`] and [`SlotDocument`] by implementing
//! [`SlotModel`] for its model, through the `impl_slot_model!` macro, then
//! names the pair with a type alias: `Ff1Format` is `SlotFormat<Ff1Model>`
//! and `Ff1Document` is `SlotDocument<Ff1Model>`. [`SlotFormat`] runs the
//! open pipeline: it decodes the container, parses the JSON text, applies
//! the game rule, checks the shape and refuses a repeated key in a keyed list
//! of the game. The summary of a slot, the party section and the inventory
//! section of its view are the same for every game.
//!
//! # Plugging in a game
//!
//! A new game supplies what the compiler does not check:
//!
//! - The model struct holds a field `tables: &'static GameTables`, because
//!   the `tables()` method of the generated impl reads it, and the
//!   `impl_slot_model!` call expands inside the module tree of the model.
//! - The model has one inherent method per [`SlotModel`] method, with the
//!   same name and signature: `new`, `document`, `rows`, `get`,
//!   `field_meta`, `collection_meta`, `summary`, `view`, `apply`,
//!   `apply_game`, `row_locked` and `field_locked`. The generated impl calls
//!   them and denies `unconditional_recursion`, so a missing one fails
//!   `cargo check`.
//! - The macro takes `game`, `capabilities` (an array expression),
//!   `keyed_lists`, `equipment_slots` and `document`.
//! - The game declares the text part `NAME`: the summary reads
//!   `character.<id>.name` for the party list (`summary`).
//! - The slot vocabulary holds `equipment.<S>` and `equipment.<S>.count`
//!   for each slot 1 to `EQUIPMENT_SLOTS`: the rules read both (`rules`).
//! - `names::computed_fields` takes exactly two computed field names.
//! - `apply_game` ends in `run_game` (`plan`), so a plan of two edits or
//!   more gets the snapshot and the rollback of the whole document.
//! - A collection only one game owns, as the FF II keywords, is wired at
//!   each place that lists collections: the model's `rows` and
//!   `collection_meta`, `SlotModel::KEYED_LISTS`, the game's `view`, its raw
//!   `apply`, `row_locked` and `apply_game`.
//!
//! The crate-private modules split the machinery as follows:
//!
//! - `access` reads values, metas, locks and row keys, and applies the raw
//!   edits.
//! - `bind` resolves a field id to gil, a character part or an item count,
//!   and defines the `CharacterPart` trait that a game's `Part` implements.
//! - `derived` reads and clamps the figures computed from a status row
//!   (`max_hp`, `max_mp`).
//! - `document` is the open slot document, [`SlotDocument`].
//! - `format` is [`SlotFormat`], the format of one game.
//! - `items` inserts and removes the rows of the item lists.
//! - `meta` builds the field and collection metas that every game shares,
//!   and holds the storage bound of a number field.
//! - `model` holds [`SlotModel`] and the `impl_slot_model!` macro, which
//!   writes the trait impl of a game model from its constants.
//! - `names` holds the section and field names every game uses, and the
//!   builders of the shared field lists. It is the one public module.
//! - `open` runs the open pipeline: decode, parse, the game check, the shape
//!   gate and the repeated-key check.
//! - `paths` holds the document paths of the three collections.
//! - `plan` checks a game edit, runs its plan and rolls it back as a whole
//!   when a step fails.
//! - `read` reads a value or finds a row or an entry by key.
//! - `rules` holds the item rules: placeholders, wearers, the floor under a
//!   count, and the insert, remove and equip plans.
//! - `stat` names the seven stats a character gains.
//! - `summary` builds the slot summary.
//! - `view` builds the shared sections of the view.
//! - `write` runs the raw `Set`: the order of its checks and the write.

pub(crate) mod access;
pub(crate) mod bind;
pub(crate) mod derived;
mod document;
mod format;
pub(crate) mod items;
pub(crate) mod meta;
mod model;
pub mod names;
pub(crate) mod open;
pub(crate) mod paths;
pub(crate) mod plan;
pub(crate) mod read;
pub(crate) mod rules;
pub(crate) mod stat;
pub(crate) mod summary;
pub(crate) mod view;
pub(crate) mod write;

pub use document::SlotDocument;
pub use format::SlotFormat;
pub use model::SlotModel;
pub(crate) use model::impl_slot_model;
