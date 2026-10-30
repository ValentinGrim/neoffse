//! The metadata of the fields and collections the six games share: the gil,
//! the `count` of an item row and the three collections. A meta describes a
//! field (label, storage bounds, game cap, tables) and no policy or game
//! rule. It reads no document.

use neoffse_core::{CollectionMeta, FieldId, FieldMeta, RowField, Shape, TableKind, Value};
use neoffse_data::GameTables;

use super::names;
use super::paths::{Collection, List};

/// Largest stored value of a number field. Neither the master data nor the
/// saves state a bound for these fields: `i32::MAX` is the clamp of the Go
/// editor and holds every counter of FF I
/// (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`, "Storage
/// bound").
pub(crate) const STORAGE_MAX: i64 = 2_147_483_647;

/// The tables that an `inventory` row key may name: items, weapons, shields,
/// armor, helmets and gloves share one `contentId` space
/// (`docs/formats/pixel-remaster.md#inventory`,
/// `docs/formats/pixel-remaster.md#id-spaces-and-tables`).
const INVENTORY_TABLES: [TableKind; 6] = [
    TableKind::Item,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
];

/// An integer field from `min` to the storage bound, capped by the game
/// constant `bound` when the tables hold it.
pub(crate) fn int(label: impl Into<String>, min: i64, bound: Option<(&GameTables, &str)>) -> FieldMeta {
    let mut meta = FieldMeta::new(label, Shape::Int { min, max: STORAGE_MAX });
    meta.cap = bound.and_then(|(tables, key)| tables.bound(key));
    meta
}

/// The gil: 0 to the storage bound, capped by `MAX_GIL`.
pub(crate) fn gil_meta(tables: &GameTables) -> FieldMeta {
    int("Gil", 0, Some((tables, "MAX_GIL")))
}

/// The `count` of an item row: 1 to the storage bound. `inventory` also takes
/// the `MAX_ITEM` cap. `key_items` takes none: `MAX_ITEM` caps the normal
/// inventory only, and the game interface keeps a key-item `count` at 1 by its
/// own rule (`docs/formats/pixel-remaster.md#inventory`,
/// `docs/formats/pixel-remaster.md#bounds-in-the-master-data`).
pub(crate) fn count_meta(list: List, tables: &GameTables) -> FieldMeta {
    let cap = (list == List::Inventory).then_some((tables, "MAX_ITEM"));
    int("Count", 1, cap)
}

/// The meta of a collection: row key, row fields and the insert and remove
/// flags. Rows keep the game's order, and the per-row locks live in the model.
pub(crate) fn collection_meta(collection: Collection, tables: &GameTables) -> CollectionMeta {
    match collection {
        Collection::Character => {
            let key = int("Character", 1, None);
            let mut meta = CollectionMeta::new(FieldId::new(names::CHARACTER), "Characters", key);
            meta.insertable = false;
            meta.removable = false;
            meta
        }
        Collection::Items(list) => {
            let (label, key_label, key_tables) = match list {
                List::Inventory => ("Items", "Item", INVENTORY_TABLES.to_vec()),
                List::KeyItems => ("Key items", "Key item", vec![TableKind::KeyItem]),
            };
            let mut key = int(key_label, 1, None);
            key.tables = key_tables;
            let mut meta = CollectionMeta::new(FieldId::new(list.section()), label, key);
            meta.fields = vec![RowField::new(names::COUNT, count_meta(list, tables), Value::Int(1))];
            meta
        }
    }
}
