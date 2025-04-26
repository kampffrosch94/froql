//! This crate is a workaround for mdbook not being able to include dependencies in tests.
use doc_comment::doc_comment;

// When running `cargo test`, rustdoc will check these files as well.
doc_comment!(include_str!("../../book/src/chapter_1.md"));
