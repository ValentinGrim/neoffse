//! Property tests of the format traits over generated edits. About half of
//! the edits are refused, on purpose: a refusal must change nothing.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::fake::{FILE, FakeDocument, FakeFormat, character, gil, hp, inventory, row};
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FileProbe, FormatError, PROBE_HEAD_LEN, RowId, SaveDocument, SaveFiles, SaveFormat, Value};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::{Index, select};

fn value() -> impl Strategy<Value = Value> {
    prop_oneof![
        (-2_000i64..20_000).prop_map(Value::Int),
        any::<bool>().prop_map(Value::Bool),
        "[a-z]{0,6}".prop_map(Value::Text),
    ]
}

fn scalar_edit() -> BoxedStrategy<Edit> {
    let known = select(FakeDocument::scalar_ids());
    let field = prop_oneof![known, Just(neoffse_core::FieldId::new("nowhere"))];
    let scalar = (field, value()).prop_map(|(field, value)| Edit::set(field, value));
    let count = (0u32..8, value()).prop_map(|(key, value)| Edit::set(row(key).field("count"), value));
    prop_oneof![scalar, count].boxed()
}

fn row_edit() -> BoxedStrategy<Edit> {
    let key = prop_oneof![0u32..8, Just(51u32), Just(201u32)];
    let collection = prop_oneof![8 => Just(inventory()), 1 => Just(character()), 1 => Just(gil())];
    let position = prop_oneof![Just(None), (0usize..7).prop_map(Some)];
    let count = prop_oneof![Just(None), (-5i64..120).prop_map(Some)];
    let insert = (collection.clone(), key.clone(), position, count).prop_map(|(collection, key, position, count)| {
        let values = count.map(|n| ("count", Value::Int(n))).into_iter().collect();
        Edit::insert(RowId::new(collection, key), position, values)
    });
    let remove = (collection, key).prop_map(|(collection, key)| Edit::remove(RowId::new(collection, key)));
    prop_oneof![insert, remove].boxed()
}

fn edit() -> BoxedStrategy<Edit> {
    prop_oneof![scalar_edit(), row_edit()].boxed()
}

fn policy() -> impl Strategy<Value = CapPolicy> {
    prop_oneof![Just(CapPolicy::Enforce), Just(CapPolicy::StorageOnly)]
}

fn change_set() -> impl Strategy<Value = ChangeSet> {
    vec(edit(), 0..8).prop_map(|edits| edits.into_iter().collect())
}

fn bytes(doc: &dyn SaveDocument) -> Vec<u8> {
    doc.encode().unwrap().get(FILE).unwrap().to_vec()
}

fn edited(edits: &[Edit], policy: CapPolicy) -> FakeDocument {
    let mut doc = FakeDocument::sample();
    for edit in edits {
        let _ = doc.apply(edit, policy);
    }
    doc
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn encode_then_open_preserves_the_document(edits in vec(edit(), 0..20), policy in policy()) {
        let mut doc = edited(&edits, policy);
        let files = doc.encode().unwrap();
        prop_assert!(doc.verify(&files).is_ok());
        let mut reopened = FakeFormat.open(&files).unwrap();
        prop_assert_eq!(reopened.view().unwrap(), doc.view().unwrap());
        prop_assert_eq!(bytes(&*reopened), bytes(&doc));
    }

    #[test]
    fn apply_then_the_inverse_restores_the_encoding(edits in vec(edit(), 0..20), policy in policy()) {
        let mut doc = FakeDocument::sample();
        for edit in &edits {
            let before = bytes(&doc);
            if let Ok(inverse) = doc.apply(edit, policy) {
                let applied = bytes(&doc);
                prop_assert!(doc.apply(&inverse, CapPolicy::StorageOnly).is_ok());
                prop_assert_eq!(bytes(&doc), before);
                prop_assert!(doc.apply(edit, CapPolicy::StorageOnly).is_ok());
                prop_assert_eq!(bytes(&doc), applied);
            }
        }
    }

    #[test]
    fn a_refused_edit_leaves_the_encoding_alone(edits in vec(edit(), 0..20), policy in policy()) {
        let mut doc = FakeDocument::sample();
        for edit in &edits {
            let before = bytes(&doc);
            if doc.apply(edit, policy).is_err() {
                prop_assert_eq!(bytes(&doc), before);
            }
        }
    }

    #[test]
    fn apply_all_is_atomic_and_invertible(prefix in vec(edit(), 0..6), set in change_set(), policy in policy()) {
        let mut doc = edited(&prefix, CapPolicy::StorageOnly);
        let before = bytes(&doc);
        match doc.apply_all(&set, policy) {
            Err(error) => {
                let rollback_failed = matches!(error, EditError::RollbackFailed { .. });
                prop_assert!(!rollback_failed);
                prop_assert_eq!(bytes(&doc), before);
            }
            Ok(inverse) => {
                let applied = bytes(&doc);
                prop_assert!(doc.apply_all(&inverse, CapPolicy::StorageOnly).is_ok());
                prop_assert_eq!(bytes(&doc), before);
                prop_assert!(doc.apply_all(&set, CapPolicy::StorageOnly).is_ok());
                prop_assert_eq!(bytes(&doc), applied);
            }
        }
    }

    #[test]
    fn any_one_changed_byte_fails_verify(edits in vec(edit(), 0..20), policy in policy(), at in any::<Index>(), delta in 1u8..=255) {
        let doc = edited(&edits, policy);
        let mut changed = bytes(&doc);
        let index = at.index(changed.len());
        changed[index] = changed[index].wrapping_add(delta);
        let mut files = SaveFiles::new();
        files.push(FILE, changed).unwrap();
        prop_assert!(doc.verify(&files).is_err());
    }

    #[test]
    fn probe_head_is_a_bounded_prefix(data in vec(any::<u8>(), 0..1_000), len in any::<u64>()) {
        let probe = FileProbe::new("x", len, &data);
        let keep = data.len().min(PROBE_HEAD_LEN);
        prop_assert_eq!(probe.head, &data[..keep]);
        prop_assert_eq!(probe.len, len);
    }

    #[test]
    fn plain_names_pass_and_names_with_a_forbidden_character_fail(
        name in "[A-Za-z0-9+=_-]{1,44}",
        forbidden in select(vec!['/', '\\', ':', '\0']),
        at in any::<Index>(),
    ) {
        prop_assert!(SaveFiles::new().push(name.clone(), vec![]).is_ok());
        let chars: Vec<char> = name.chars().collect();
        let split = at.index(chars.len() + 1);
        let mut broken: String = chars[..split].iter().collect();
        broken.push(forbidden);
        broken.extend(&chars[split..]);
        let result = SaveFiles::new().push(broken, vec![]);
        let refused = matches!(result, Err(FormatError::InvalidFileName { .. }));
        prop_assert!(refused);
    }

    #[test]
    fn the_policy_decides_the_cap_and_nothing_else(number in -2_000i64..20_000) {
        let edit = Edit::set(hp(1), Value::Int(number));
        let enforced = FakeDocument::sample().apply(&edit, CapPolicy::Enforce);
        let verdict = match &enforced {
            Ok(_) => "ok",
            Err(EditError::AboveCap { .. }) => "above-cap",
            Err(EditError::OutOfRange { .. }) => "out-of-range",
            Err(_) => "other",
        };
        let expected = match number {
            0..=999 => "ok",
            1_000..=9_999 => "above-cap",
            _ => "out-of-range",
        };
        prop_assert_eq!(verdict, expected);
        let expert = FakeDocument::sample().apply(&edit, CapPolicy::StorageOnly);
        prop_assert_eq!(expert.is_ok(), (0..=9_999).contains(&number));
    }
}
