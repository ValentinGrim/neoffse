//! Property tests of `Document` over generated trees of nested string documents.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Step, changed_span, embed_with, segments};
use neoffse_format_pr::document::{Document, Scalar};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::{Index, select};

/// Leaf spellings that an untouched value must keep as written. Two of them,
/// `1E5` and `"a\/b"`, change when `serde_json::Value` re-serializes them
/// (`1e+5` and `"a/b"`). The other six pass through `Value` unchanged under
/// the crate's features and pin the plain pass-through.
const ODD: [&str; 8] = ["1E5", "-0", "1.50", "58.0", "\"a\\/b\"", "\"é\"", "true", "null"];

#[derive(Debug, Clone)]
enum Tree {
    Int(i64),
    Odd(&'static str),
    Obj(Vec<Tree>),
    Arr(Vec<Tree>),
    /// An object rendered as text and embedded as a string. The flag writes
    /// each quote of the embedded text as `"` instead of `\"`.
    Doc(bool, Vec<Tree>),
}

fn tree() -> BoxedStrategy<Tree> {
    let leaf = prop_oneof![any::<i64>().prop_map(Tree::Int), select(ODD.to_vec()).prop_map(Tree::Odd)];
    leaf.prop_recursive(4, 32, 4, |inner| {
        prop_oneof![
            vec(inner.clone(), 0..4).prop_map(Tree::Obj),
            vec(inner.clone(), 0..4).prop_map(Tree::Arr),
            (any::<bool>(), vec(inner, 0..4)).prop_map(|(flag, members)| Tree::Doc(flag, members)),
        ]
    })
    .boxed()
}

fn roots() -> impl Strategy<Value = Tree> {
    vec(tree(), 0..4).prop_map(Tree::Obj)
}

/// What a render walks past.
#[derive(Default)]
struct Shape {
    /// Every value, the root excluded.
    all: Vec<Vec<Step>>,
    ints: Vec<Vec<Step>>,
    strings: Vec<Vec<Step>>,
    /// Replaces the `n`th integer in render order.
    replace: Option<(usize, i64)>,
    /// Writes every embedded quote as `\"`, the form an edit produces.
    canonical_quotes: bool,
}

fn render_members(members: &[Tree], keyed: bool, spaced: bool, path: &mut Vec<Step>, shape: &mut Shape) -> String {
    let separator = if spaced { ", " } else { "," };
    let colon = if spaced { ": " } else { ":" };
    let parts: Vec<String> = members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            path.push(if keyed { Step::Key(format!("k{index}")) } else { Step::Index(index) });
            let text = render(member, spaced, path, shape);
            path.pop();
            if keyed { format!("\"k{index}\"{colon}{text}") } else { text }
        })
        .collect();
    parts.join(separator)
}

fn render(tree: &Tree, spaced: bool, path: &mut Vec<Step>, shape: &mut Shape) -> String {
    shape.all.push(path.clone());
    match tree {
        Tree::Int(value) => {
            let ordinal = shape.ints.len();
            shape.ints.push(path.clone());
            match shape.replace {
                Some((target, new)) if target == ordinal => new.to_string(),
                _ => value.to_string(),
            }
        }
        Tree::Odd(text) => {
            if text.starts_with('"') {
                shape.strings.push(path.clone());
            }
            (*text).to_owned()
        }
        Tree::Obj(members) => format!("{{{}}}", render_members(members, true, spaced, path, shape)),
        Tree::Arr(members) => format!("[{}]", render_members(members, false, spaced, path, shape)),
        Tree::Doc(unicode_quote, members) => {
            shape.strings.push(path.clone());
            path.push(Step::Doc);
            let inner = format!("{{{}}}", render_members(members, true, spaced, path, shape));
            path.pop();
            embed_with(&inner, *unicode_quote && !shape.canonical_quotes)
        }
    }
}

fn render_root(tree: &Tree, spaced: bool, shape: &mut Shape) -> String {
    render(tree, spaced, &mut Vec::new(), shape)
}

fn segment_strategy() -> impl Strategy<Value = Step> {
    prop_oneof![
        select(vec!["a", "b", "c", "k0", "k1"]).prop_map(|key| Step::Key(key.to_owned())),
        (0usize..4).prop_map(Step::Index),
        Just(Step::Doc),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn untouched_documents_round_trip_in_any_form(tree in roots(), spaced in any::<bool>()) {
        let mut shape = Shape::default();
        let text = render_root(&tree, spaced, &mut shape);
        let mut document = Document::parse(&text).unwrap();
        for path in &shape.all {
            document.raw(&segments(path)).unwrap();
        }
        prop_assert!(document.to_json() == text);
    }

    #[test]
    fn an_integer_edit_changes_only_that_integer(tree in roots(), pick in any::<Index>(), new in any::<i64>()) {
        let mut shape = Shape { canonical_quotes: true, ..Shape::default() };
        let text = render_root(&tree, false, &mut shape);
        prop_assume!(!shape.ints.is_empty());
        let ordinal = pick.index(shape.ints.len());
        let path = shape.ints[ordinal].clone();

        let mut document = Document::parse(&text).unwrap();
        document.set(&segments(&path), &Scalar::Int(new)).unwrap();
        let output = document.to_json();

        let mut edited = Shape { replace: Some((ordinal, new)), canonical_quotes: true, ..Shape::default() };
        let expected = render_root(&tree, false, &mut edited);
        prop_assert!(output == expected);
        let (old_span, new_span) = changed_span(&text, &output);
        let digits = |span: &str| span.bytes().all(|byte| byte.is_ascii_digit() || byte == b'-');
        prop_assert!(digits(old_span) && digits(new_span));
    }

    #[test]
    fn text_set_round_trips_through_get_and_escapes_like_serde_json(
        tree in roots(),
        pick in any::<Index>(),
        content in any::<String>(),
    ) {
        let mut shape = Shape::default();
        let text = render_root(&tree, true, &mut shape);
        prop_assume!(!shape.strings.is_empty());
        let path = shape.strings[pick.index(shape.strings.len())].clone();
        let steps = segments(&path);

        let mut document = Document::parse(&text).unwrap();
        document.set(&steps, &Scalar::Text(content.clone())).unwrap();
        let token = serde_json::to_string(&content).unwrap();
        prop_assert!(document.raw(&steps).unwrap() == token);
        let mut fresh = Document::parse(&document.to_json()).unwrap();
        prop_assert!(fresh.get(&steps).unwrap() == Scalar::Text(content));
    }

    #[test]
    fn parse_and_navigation_never_panic_on_garbage(
        text in prop_oneof![
            "\\PC{0,200}",
            "[a-d{}\\[\\]:,\"\\\\0-9 ]{0,80}",
            roots().prop_map(|tree| render_root(&tree, true, &mut Shape::default())),
        ],
        paths in vec(vec(segment_strategy(), 0..6), 0..4),
    ) {
        if let Ok(mut document) = Document::parse(&text) {
            for path in &paths {
                let steps = segments(path);
                let _ = document.raw(&steps).map(|raw| raw.len());
                let _ = document.get(&steps);
                let _ = document.set_raw(&steps, "1");
            }
            let _ = document.to_json();
        }
    }
}
