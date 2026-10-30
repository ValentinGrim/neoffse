//! The section names every Pixel Remaster slot uses, the ones that build the
//! first key of a `FieldId`.

/// Section of the gil counter, `userData.owendGil`.
pub const GIL: &str = "gil";
/// Section of the party characters, one row per `ownedCharacterList` element.
pub const CHARACTER: &str = "character";
/// Section of the normal item rows (items and equipment), `normalOwnedItemList`.
pub const INVENTORY: &str = "inventory";
/// Section of the key item rows, `importantOwendItemList`.
pub const KEY_ITEMS: &str = "key_items";
/// Character name, `name`.
pub const NAME: &str = "name";
/// Count of an inventory row or of an equipment slot.
pub const COUNT: &str = "count";
