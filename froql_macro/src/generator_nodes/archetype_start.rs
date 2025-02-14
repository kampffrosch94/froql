use crate::Checks;

use super::relation_helper::{
    relation_helpers_init_and_set_col, relation_helpers_set_rows, RelationHelperInfo,
    UnrelationHelperInfo,
};
use super::relation_join::{insert_checks, insert_optional_comps};
use super::GeneratorNode;
use std::fmt::Write;
use std::ops::Range;

#[derive(Debug)]
pub struct ArchetypeStart {
    pub var: isize,
    pub components: Range<usize>,
    pub opt_components: Vec<(String, usize)>,
    pub relation_helpers: Vec<RelationHelperInfo>,
    pub unrelation_helpers: Vec<UnrelationHelperInfo>,
    pub checks: Checks,
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
        relation_helpers_init_and_set_col(
            prepend,
            append,
            &self.relation_helpers,
            &self.unrelation_helpers,
        );

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
    }} else {{"
        )
        .unwrap();

        if self.checks.is_empty() {
            relation_helpers_set_rows(append, &self.relation_helpers, &self.unrelation_helpers);

            write!(
                append,
                "
        current_step += 1;"
            )
            .unwrap();
        } else {
            append.push_str(
                "
        let id = a_refs[CURRENT_VAR].entities[a_rows[CURRENT_VAR].as_index()];",
            );
            insert_checks(
                append,
                &self.checks.unequals,
                &self.checks.rel_constraints,
                &self.checks.unrel_constraints,
            );
            append.push_str(
                "
        {} else {",
            );
            relation_helpers_set_rows(append, &self.relation_helpers, &self.unrelation_helpers);

            append.push_str(
                "
            current_step += 1;
        }",
            );
        }

        write!(
            append,
            "
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
            relation_helpers: vec![],
            unrelation_helpers: vec![],
            checks: Checks {
                unequals: vec![],
                rel_constraints: vec![],
                unrel_constraints: vec![],
            },
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(0, &mut prepend, &mut append);
        assert_eq!(2, r);
        assert_eq!(prepend, "");
        insta::assert_snapshot!(append, @r#"
        0 => {
            const CURRENT_VAR: usize = 0;
            const CURRENT_VAR_COMPONENTS: ::std::ops::Range<usize> = 0..2;
            let next_index = &mut a_next_indexes[CURRENT_VAR];
            let archetype_ids = &archetype_id_sets[CURRENT_VAR];
            *next_index = next_index.wrapping_add(1);
            if *next_index >= archetype_ids.len() {
                return None;
            }
            let next_id = archetype_ids[*next_index];

            // gets rolled over to 0 by wrapping_add
            a_rows[CURRENT_VAR] = ArchetypeRow(u32::MAX);
            let a_ref = &mut a_refs[CURRENT_VAR];
            *a_ref = &bk.archetypes[next_id.as_index()];
            a_ref.find_multiple_columns(
                &components_0,
                &mut col_indexes[CURRENT_VAR_COMPONENTS],
            );
            a_max_rows[CURRENT_VAR] = a_ref.entities.len() as u32;
            current_step += 1;
        }

        // next row in archetype
        1 => {
            const CURRENT_VAR: usize = 0;
            let row_counter = &mut a_rows[CURRENT_VAR].0;
            let max_row = a_max_rows[CURRENT_VAR];
            // rolls over to 0 for u32::MAX, which is our start value
            *row_counter = row_counter.wrapping_add(1);

            if *row_counter >= max_row {
                current_step -= 1;
            } else {
                current_step += 1;
            }
        }
        "#);
    }

    #[test]
    fn check_unequal() {
        let gen = ArchetypeStart {
            var: 0,
            components: 0..2,
            opt_components: vec![],
            relation_helpers: vec![],
            unrelation_helpers: vec![],
            checks: Checks {
                unequals: vec![(0, 1)],
                rel_constraints: vec![],
                unrel_constraints: vec![],
            },
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(0, &mut prepend, &mut append);
        assert_eq!(2, r);
        assert_eq!(prepend, "");
        insta::assert_snapshot!(append, @r#"
        0 => {
            const CURRENT_VAR: usize = 0;
            const CURRENT_VAR_COMPONENTS: ::std::ops::Range<usize> = 0..2;
            let next_index = &mut a_next_indexes[CURRENT_VAR];
            let archetype_ids = &archetype_id_sets[CURRENT_VAR];
            *next_index = next_index.wrapping_add(1);
            if *next_index >= archetype_ids.len() {
                return None;
            }
            let next_id = archetype_ids[*next_index];

            // gets rolled over to 0 by wrapping_add
            a_rows[CURRENT_VAR] = ArchetypeRow(u32::MAX);
            let a_ref = &mut a_refs[CURRENT_VAR];
            *a_ref = &bk.archetypes[next_id.as_index()];
            a_ref.find_multiple_columns(
                &components_0,
                &mut col_indexes[CURRENT_VAR_COMPONENTS],
            );
            a_max_rows[CURRENT_VAR] = a_ref.entities.len() as u32;
            current_step += 1;
        }

        // next row in archetype
        1 => {
            const CURRENT_VAR: usize = 0;
            let row_counter = &mut a_rows[CURRENT_VAR].0;
            let max_row = a_max_rows[CURRENT_VAR];
            // rolls over to 0 for u32::MAX, which is our start value
            *row_counter = row_counter.wrapping_add(1);

            if *row_counter >= max_row {
                current_step -= 1;
            } else {
                let id = a_refs[CURRENT_VAR].entities[a_rows[CURRENT_VAR].as_index()];
                    if
                        (::std::ptr::eq(a_refs[0], a_refs[1])
                         && a_rows[0] == a_rows[1])
                {} else {
                    current_step += 1;
                }
            }
        }
        "#);
    }
}
