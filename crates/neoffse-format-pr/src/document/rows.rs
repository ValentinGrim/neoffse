//! Row operations: insert into and remove from a JSON array.

use super::{Document, DocumentError, Node, Scalar, Segment, check_depth, parse_value, render_path, split_outer};

fn out_of_range(path: &[Segment<'_>], index: usize, len: usize) -> DocumentError {
    DocumentError::IndexOutOfRange {
        path: render_path(path),
        index,
        len,
    }
}

impl Document {
    /// The number of elements of the array at `path`. A path may end in
    /// [`Segment::Doc`] when the string holds an array document. Like
    /// [`get`](Self::get), it parses levels and never marks one dirty.
    ///
    /// # Errors
    ///
    /// `TooDeep` above [`MAX_DEPTH`](super::MAX_DEPTH) segments, then any step
    /// error, then `WrongKind` with `expected: "array"` when the value is no
    /// array.
    ///
    /// # Examples
    ///
    /// ```
    /// use neoffse_format_pr::document::{Document, Segment};
    ///
    /// let mut document = Document::parse(r#"{"list": [1, 2, 3]}"#)?;
    /// assert_eq!(document.array_len(&[Segment::Key("list")])?, 3);
    /// # Ok::<(), neoffse_format_pr::document::DocumentError>(())
    /// ```
    pub fn array_len(&mut self, path: &[Segment<'_>]) -> Result<usize, DocumentError> {
        check_depth(path)?;
        Ok(self.root.walk(path, false)?.array_items(path)?.len())
    }

    /// Inserts `value` before element `index` of the array at `path`
    /// (see [`insert_raw`](Self::insert_raw)). The text is the one
    /// [`set`](Self::set) writes: an integer is plain decimal, a string is
    /// escaped by `serde_json`.
    ///
    /// # Errors
    ///
    /// In this order: `TooDeep`, `NonFiniteFloat` for a float that is NaN or
    /// infinite (before any step of the path runs, so a path that does not
    /// exist still gives it), then the errors of
    /// [`insert_raw`](Self::insert_raw) after its `InvalidJson`: any step
    /// error of the path, `WrongKind`, `IndexOutOfRange`. The document stays
    /// as it was.
    ///
    /// # Examples
    ///
    /// ```
    /// use neoffse_format_pr::document::{Document, Scalar, Segment};
    ///
    /// let mut document = Document::parse(r#"{"list": [1, 3]}"#)?;
    /// document.insert(&[Segment::Key("list")], 1, &Scalar::Int(2))?;
    /// assert_eq!(document.to_json(), r#"{"list":[1,2,3]}"#);
    /// # Ok::<(), neoffse_format_pr::document::DocumentError>(())
    /// ```
    pub fn insert(&mut self, path: &[Segment<'_>], index: usize, value: &Scalar) -> Result<(), DocumentError> {
        check_depth(path)?;
        let json = value.json_text(path)?;
        self.insert_raw(path, index, &json)
    }

    /// Inserts `json`, one valid JSON value of any kind, before element
    /// `index` of the array at `path`. `index == len` appends. Whitespace
    /// around `json` is dropped, its inner form is kept. The array and every
    /// level above it turn dirty and are written compact on the next
    /// [`to_json`](Self::to_json); the other elements keep their text.
    ///
    /// # Errors
    ///
    /// `TooDeep`, `InvalidJson` for `json`, then any step error of the path,
    /// `WrongKind` with `expected: "array"` when the value is no array, and
    /// `IndexOutOfRange` when `index` exceeds the length. The document stays
    /// as it was.
    ///
    /// # Examples
    ///
    /// ```
    /// use neoffse_format_pr::document::{Document, Segment};
    ///
    /// let mut document = Document::parse(r#"{"list": [1.50, 2]}"#)?;
    /// let list = [Segment::Key("list")];
    /// document.insert_raw(&list, 2, " {\"a\":  1} ")?;
    /// // The array is compact; every element keeps its text.
    /// assert_eq!(document.to_json(), r#"{"list":[1.50,2,{"a":  1}]}"#);
    ///
    /// assert!(document.insert_raw(&list, 9, "0").is_err());
    /// # Ok::<(), neoffse_format_pr::document::DocumentError>(())
    /// ```
    pub fn insert_raw(&mut self, path: &[Segment<'_>], index: usize, json: &str) -> Result<(), DocumentError> {
        check_depth(path)?;
        let (_, body, _) = split_outer(json);
        let value = parse_value(body, json, path)?;
        let len = self.root.walk(path, false)?.array_items(path)?.len();
        if index > len {
            return Err(out_of_range(path, index, len));
        }
        let array = self.root.walk(path, true)?;
        array.touch();
        let items = array.array_items(path)?;
        if index > items.len() {
            return Err(out_of_range(path, index, items.len()));
        }
        items.insert(index, Node::new(value));
        Ok(())
    }

    /// Deletes element `index` of the array at `path` and returns its JSON
    /// text: the stored text when the element is clean, the re-serialized text
    /// when an earlier edit touched it (the text [`raw`](Self::raw) would have
    /// returned). The array and every level above it turn dirty.
    ///
    /// # Errors
    ///
    /// `TooDeep`, then any step error of the path, `WrongKind` with
    /// `expected: "array"` when the value is no array, and `IndexOutOfRange`
    /// when `index` is not below the length. The document stays as it was.
    ///
    /// # Examples
    ///
    /// ```
    /// use neoffse_format_pr::document::{Document, Segment};
    ///
    /// let mut document = Document::parse(r#"{"list": [1.50, 2]}"#)?;
    /// let removed = document.remove(&[Segment::Key("list")], 0)?;
    /// assert_eq!(removed, "1.50");
    /// assert_eq!(document.to_json(), r#"{"list":[2]}"#);
    /// # Ok::<(), neoffse_format_pr::document::DocumentError>(())
    /// ```
    pub fn remove(&mut self, path: &[Segment<'_>], index: usize) -> Result<String, DocumentError> {
        check_depth(path)?;
        let len = self.root.walk(path, false)?.array_items(path)?.len();
        if index >= len {
            return Err(out_of_range(path, index, len));
        }
        let array = self.root.walk(path, true)?;
        array.touch();
        let items = array.array_items(path)?;
        if index >= items.len() {
            return Err(out_of_range(path, index, items.len()));
        }
        Ok(items.remove(index).text().into_owned())
    }
}
