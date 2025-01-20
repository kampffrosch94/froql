use std::fmt::Debug;

pub mod archetype_start;
pub mod invar_start;
pub mod relation_join;

pub trait GeneratorNode: Debug {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String) -> usize;
}
