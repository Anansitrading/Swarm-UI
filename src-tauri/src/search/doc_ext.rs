use tantivy::schema::{Field, OwnedValue};
use tantivy::{DateTime, TantivyDocument};

/// Extension trait on [`TantivyDocument`] for typed field extraction.
///
/// Handles [`OwnedValue`] enum matching in one place so callers
/// get `Option<T>` instead of pattern-matching every access.
pub trait DocExt {
    fn get_str(&self, field: Field) -> Option<&str>;
    fn get_u64_val(&self, field: Field) -> Option<u64>;
    fn get_bool_val(&self, field: Field) -> Option<bool>;
    fn get_date_val(&self, field: Field) -> Option<DateTime>;
}

impl DocExt for TantivyDocument {
    fn get_str(&self, field: Field) -> Option<&str> {
        self.get_first(field).and_then(|v| match v {
            OwnedValue::Str(s) => Some(s.as_str()),
            _ => None,
        })
    }

    fn get_u64_val(&self, field: Field) -> Option<u64> {
        self.get_first(field).and_then(|v| match v {
            OwnedValue::U64(n) => Some(*n),
            _ => None,
        })
    }

    fn get_bool_val(&self, field: Field) -> Option<bool> {
        self.get_first(field).and_then(|v| match v {
            OwnedValue::Bool(b) => Some(*b),
            _ => None,
        })
    }

    fn get_date_val(&self, field: Field) -> Option<DateTime> {
        self.get_first(field).and_then(|v| match v {
            OwnedValue::Date(d) => Some(*d),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::schema::IndexSchema;

    #[test]
    fn test_get_str_extracts_text_field() {
        let idx = IndexSchema::new();
        let doc = TantivyDocument::parse_json(
            &idx.schema,
            r#"{"session_id": ["abc-123"], "doc_type": ["session"], "model": ["claude-opus-4-6"]}"#,
        )
        .unwrap();

        assert_eq!(doc.get_str(idx.session_id), Some("abc-123"));
        assert_eq!(doc.get_str(idx.model), Some("claude-opus-4-6"));
        // Wrong type: u64 field returns None for get_str
        assert_eq!(doc.get_str(idx.message_count), None);
    }

    #[test]
    fn test_get_u64_val_extracts_numeric_field() {
        let idx = IndexSchema::new();
        let doc = TantivyDocument::parse_json(
            &idx.schema,
            r#"{"session_id": ["s1"], "doc_type": ["session"], "message_count": [42], "total_tokens": [9001]}"#,
        )
        .unwrap();

        assert_eq!(doc.get_u64_val(idx.message_count), Some(42));
        assert_eq!(doc.get_u64_val(idx.total_tokens), Some(9001));
        // Wrong type: text field returns None for get_u64_val
        assert_eq!(doc.get_u64_val(idx.session_id), None);
    }

    #[test]
    fn test_get_bool_val_extracts_boolean_field() {
        let idx = IndexSchema::new();
        let doc = TantivyDocument::parse_json(
            &idx.schema,
            r#"{"session_id": ["s1"], "doc_type": ["session"], "archived": [false], "file_exists": [true], "has_tool_use": [true]}"#,
        )
        .unwrap();

        assert_eq!(doc.get_bool_val(idx.archived), Some(false));
        assert_eq!(doc.get_bool_val(idx.file_exists), Some(true));
        assert_eq!(doc.get_bool_val(idx.has_tool_use), Some(true));
        // Wrong type: text field returns None for get_bool_val
        assert_eq!(doc.get_bool_val(idx.session_id), None);
    }

    #[test]
    fn test_get_date_val_extracts_date_field() {
        let idx = IndexSchema::new();
        let doc = TantivyDocument::parse_json(
            &idx.schema,
            r#"{"session_id": ["s1"], "doc_type": ["session"], "created_at": ["2026-02-18T12:00:00Z"]}"#,
        )
        .unwrap();

        let date = doc.get_date_val(idx.created_at);
        assert!(date.is_some(), "created_at should parse as Date");
        // Missing field returns None
        assert_eq!(doc.get_date_val(idx.modified_at), None);
        // Wrong type: text field returns None for get_date_val
        assert_eq!(doc.get_date_val(idx.session_id), None);
    }
}
