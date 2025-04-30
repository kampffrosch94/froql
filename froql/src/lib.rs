#![allow(clippy::needless_return)]
#![allow(clippy::new_without_default)]

pub mod archetype;
pub mod bookkeeping;
pub mod component;
pub mod entity_store;
mod layout_vec;
pub mod query_helper;
pub mod relation;
pub mod relation_vec;
mod util;
pub mod world;
pub use froql_macro::query;
pub mod entity_view_deferred;
pub mod entity_view_mut;
