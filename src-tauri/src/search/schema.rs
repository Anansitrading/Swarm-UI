use tantivy::schema::{
    DateOptions, Field, NumericOptions, Schema, SchemaBuilder, TextFieldIndexing, TextOptions,
    FAST, STORED, STRING,
};

/// Schema version — bump forces full reindex.
pub const SCHEMA_VERSION: u64 = 1;

/// Pre-cached field handles for the Tantivy index schema.
///
/// Two document types share a single index, discriminated by `doc_type`:
/// - "session": one per JSONL file (20 fields)
/// - "message": one per content block (10 fields)
#[derive(Debug, Clone)]
pub struct IndexSchema {
    pub schema: Schema,

    // -- Shared fields --
    pub session_id: Field,
    pub doc_type: Field,

    // -- Session fields --
    pub project_path: Field,
    pub project_raw: Field,
    pub summary: Field,
    pub first_prompt: Field,
    pub git_branch: Field,
    pub model: Field,
    pub status: Field,
    pub jsonl_path: Field,
    pub message_count: Field,
    pub input_tokens: Field,
    pub output_tokens: Field,
    pub total_tokens: Field,
    pub created_at: Field,
    pub modified_at: Field,
    pub archived: Field,
    pub file_exists: Field,
    pub has_tool_use: Field,
    pub turn_depth: Field,

    // -- Message fields --
    pub role: Field,
    pub content: Field,
    pub content_stored: Field,
    pub content_type: Field,
    pub timestamp: Field,
    pub turn_index: Field,
    pub block_index: Field,
    pub msg_project: Field,
}

impl IndexSchema {
    /// Build the schema and cache all field handles.
    pub fn new() -> Self {
        let mut builder = SchemaBuilder::new();

        // -- Shared fields --
        let session_id = builder.add_text_field("session_id", STRING | FAST | STORED);
        let doc_type = builder.add_text_field("doc_type", STRING | FAST | STORED);

        // -- Session fields --
        // project_path: TEXT STORED (tokenized for fuzzy search)
        let text_stored = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("default")
                    .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();
        let project_path = builder.add_text_field("project_path", text_stored);

        // project_raw: STRING FAST STORED (untokenized for exact grouping)
        let project_raw = builder.add_text_field("project_raw", STRING | FAST | STORED);

        // summary, first_prompt: TEXT STORED (BM25 searchable + retrievable)
        let text_stored_searchable = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("default")
                    .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();
        let summary = builder.add_text_field("summary", text_stored_searchable.clone());
        let first_prompt = builder.add_text_field("first_prompt", text_stored_searchable);

        let git_branch = builder.add_text_field("git_branch", STRING | FAST | STORED);
        let model = builder.add_text_field("model", STRING | FAST | STORED);
        let status = builder.add_text_field("status", STRING | FAST | STORED);
        let jsonl_path = builder.add_text_field("jsonl_path", STRING | STORED);

        // Numeric fields: FAST STORED
        let u64_fast_stored = NumericOptions::default().set_fast().set_stored();
        let message_count = builder.add_u64_field("message_count", u64_fast_stored.clone());
        let input_tokens = builder.add_u64_field("input_tokens", u64_fast_stored.clone());
        let output_tokens = builder.add_u64_field("output_tokens", u64_fast_stored.clone());
        let total_tokens = builder.add_u64_field("total_tokens", u64_fast_stored.clone());

        // Date fields: FAST STORED
        let date_fast_stored = DateOptions::default().set_fast().set_stored();
        let created_at = builder.add_date_field("created_at", date_fast_stored.clone());
        let modified_at = builder.add_date_field("modified_at", date_fast_stored.clone());

        // Bool fields: FAST STORED
        let bool_fast_stored = NumericOptions::default().set_fast().set_stored();
        let archived = builder.add_bool_field("archived", bool_fast_stored.clone());
        let file_exists = builder.add_bool_field("file_exists", bool_fast_stored.clone());
        let has_tool_use = builder.add_bool_field("has_tool_use", bool_fast_stored);

        let turn_depth = builder.add_u64_field("turn_depth", u64_fast_stored);

        // -- Message fields --
        let role = builder.add_text_field("role", STRING | FAST | STORED);

        // content: TEXT only (NOT STORED) — saves ~600MB disk
        let text_only = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("default")
                .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions),
        );
        let content = builder.add_text_field("content", text_only);

        let content_stored = builder.add_text_field("content_stored", STRING | STORED);
        let content_type = builder.add_text_field("content_type", STRING | FAST | STORED);

        let timestamp = builder.add_date_field("timestamp", date_fast_stored);

        let turn_index = builder.add_u64_field("turn_index", NumericOptions::default().set_fast().set_stored());
        let block_index = builder.add_u64_field("block_index", NumericOptions::default().set_fast().set_stored());

        // msg_project: STRING FAST (denormalized for filtering, not stored)
        let msg_project = builder.add_text_field("msg_project", STRING | FAST);

        let schema = builder.build();

        IndexSchema {
            schema,
            session_id,
            doc_type,
            project_path,
            project_raw,
            summary,
            first_prompt,
            git_branch,
            model,
            status,
            jsonl_path,
            message_count,
            input_tokens,
            output_tokens,
            total_tokens,
            created_at,
            modified_at,
            archived,
            file_exists,
            has_tool_use,
            turn_depth,
            role,
            content,
            content_stored,
            content_type,
            timestamp,
            turn_index,
            block_index,
            msg_project,
        }
    }

    /// Total number of fields in the schema.
    pub fn field_count(&self) -> usize {
        28
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_has_28_fields() {
        let idx = IndexSchema::new();
        // SchemaBuilder assigns sequential field IDs starting at 0
        assert_eq!(idx.schema.num_fields(), 28);
        assert_eq!(idx.field_count(), 28);
    }

    #[test]
    fn test_schema_version_is_1() {
        assert_eq!(SCHEMA_VERSION, 1);
    }

    #[test]
    fn test_content_field_is_not_stored() {
        let idx = IndexSchema::new();
        let entry = idx.schema.get_field_entry(idx.content);
        // content should be indexed (TEXT) but NOT stored
        assert!(entry.is_indexed());
        assert!(
            !entry.is_stored(),
            "content field must NOT be stored to save disk"
        );
    }

    #[test]
    fn test_session_id_is_fast_and_stored() {
        let idx = IndexSchema::new();
        let entry = idx.schema.get_field_entry(idx.session_id);
        assert!(entry.is_indexed());
        assert!(
            entry.is_stored(),
            "session_id must be stored for retrieval"
        );
        // Verify field name round-trips
        assert_eq!(entry.name(), "session_id");
    }
}
