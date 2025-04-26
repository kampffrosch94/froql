//! This crate is a workaround for mdbook not being able to include dependencies in tests.
use doc_comment::doc_comment;

// When running `cargo test`, rustdoc will check these files as well.
doc_comment!(include_str!("../../book/src/entities.md"));
doc_comment!(include_str!("../../book/src/index.md"));
doc_comment!(include_str!("../../book/src/queries.md"));
doc_comment!(include_str!("../../book/src/relations.md"));
doc_comment!(include_str!("../../book/src/singletons.md"));
