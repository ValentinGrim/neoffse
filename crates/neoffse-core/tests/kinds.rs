//! Capabilities, table kinds and the cap policy.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::{BTreeSet, HashSet};

use neoffse_core::{CapPolicy, Capability, TableKind};

#[test]
fn a_capability_is_a_set_key() {
    let set: BTreeSet<Capability> = [Capability::Party, Capability::Inventory, Capability::Party, Capability::Magic]
        .into_iter()
        .collect();
    assert_eq!(set.len(), 3);
    let ordered: Vec<Capability> = set.into_iter().collect();
    assert_eq!(ordered, [Capability::Party, Capability::Inventory, Capability::Magic]);
}

#[test]
fn table_kinds_are_distinct_and_hashable() {
    let kinds = [
        TableKind::Item,
        TableKind::KeyItem,
        TableKind::Weapon,
        TableKind::Shield,
        TableKind::Armor,
        TableKind::Helmet,
        TableKind::Gloves,
        TableKind::Ability,
        TableKind::Job,
        TableKind::Command,
        TableKind::Map,
        TableKind::Monster,
        TableKind::Keyword,
        TableKind::MagicStone,
        TableKind::CommandContent,
    ];
    let set: HashSet<TableKind> = kinds.into_iter().collect();
    assert_eq!(set.len(), 15);
}

fn take(capability: Capability, table: TableKind, policy: CapPolicy) -> (Capability, TableKind, CapPolicy) {
    (capability, table, policy)
}

#[test]
fn kinds_are_copy() {
    let capability = Capability::Party;
    let table = TableKind::Item;
    let policy = CapPolicy::Enforce;
    let first = take(capability, table, policy);
    let second = take(capability, table, policy);
    assert_eq!(first, second);
}

#[test]
fn the_default_policy_enforces_the_cap() {
    assert_eq!(CapPolicy::default(), CapPolicy::Enforce);
    assert_ne!(CapPolicy::Enforce, CapPolicy::StorageOnly);
}
