use super::relation_join::insert_optional_comps;
use super::GeneratorNode;
use std::fmt::Write;
use std::ops::Range;

#[derive(Debug)]
pub struct ArchetypeStart {
    pub var: isize,
    pub components: Range<usize>,
    pub opt_components: Vec<(String, usize)>,
}

impl GeneratorNode for ArchetypeStart {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String) -> usize {
        let first = self.var;
        let Range { start, end } = &self.components;
        write!(
            append,
            "
{step} => {{
    const CURRENT_VAR: usize = {first};
    const CURRENT_VAR_COMPONENTS: ::std::ops::Range<usize> = {start}..{end};
    let next_index = &mut a_next_indexes[CURRENT_VAR];
    let archetype_ids = &archetype_id_sets[CURRENT_VAR];
    *next_index = next_index.wrapping_add(1);
    if *next_index >= archetype_ids.len() {{
        return None;
    }}
    let next_id = archetype_ids[*next_index];

    // gets rolled over to 0 by wrapping_add
    a_rows[CURRENT_VAR] = ArchetypeRow(u32::MAX);
    let a_ref = &mut a_refs[CURRENT_VAR];
    *a_ref = &bk.archetypes[next_id.as_index()];
    a_ref.find_multiple_columns(
        &components_{first},
        &mut col_indexes[CURRENT_VAR_COMPONENTS],
    );
    a_max_rows[CURRENT_VAR] = a_ref.entities.len() as u32;"
        )
        .unwrap();

        // handle optional components
        insert_optional_comps(prepend, append, &self.opt_components);

        write!(
            append,
            "
    current_step += 1;
}}
"
        )
        .unwrap();

        // get row from first archetype
        let next_step = step + 1;
        write!(
            append,
            "
// next row in archetype
{next_step} => {{
    const CURRENT_VAR: usize = {first};
    let row_counter = &mut a_rows[CURRENT_VAR].0;
    let max_row = a_max_rows[CURRENT_VAR];
    // rolls over to 0 for u32::MAX, which is our start value
    *row_counter = row_counter.wrapping_add(1);

    if *row_counter >= max_row {{
        current_step -= 1;
    }} else {{
        current_step += 1;
    }}
}}
"
        )
        .unwrap();
        return step + 2;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn normal() {
        let gen = ArchetypeStart {
            var: 0,
            components: 0..2,
            opt_components: vec![],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(0, &mut prepend, &mut append);
        assert_eq!(2, r);
        assert_eq!(prepend, "");
        insta::assert_snapshot!(append);
    }
}
