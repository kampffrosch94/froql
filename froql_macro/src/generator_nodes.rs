use std::fmt::Debug;

pub(crate) mod relation_join;

pub(crate) trait GeneratorNode: Debug {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String) -> usize;
}
