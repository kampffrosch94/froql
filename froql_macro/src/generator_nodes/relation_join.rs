use super::GeneratorNode;
use std::fmt::Write;
use std::ops::Range;

#[derive(Debug)]
pub struct RelationJoin {
    /// index of the component of `old` where the relation resides
    pub relation_comp: usize,
    pub old: isize,
    pub new: isize,
    pub new_components: Range<usize>,
    pub unequalities: Vec<(isize, isize)>,
    pub rel_constraints: Vec<(usize, isize, isize)>,
}

impl GeneratorNode for RelationJoin {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String) -> usize {
        let old = self.old;
        let new = self.new;
        let comp = self.relation_comp;
        let Range { start, end } = &self.new_components;
        write!(prepend, "\nlet mut rel_index_{step} = 0;").unwrap();
        write!(
            append,
            "
// follow relation
{step} => {{
    const CURRENT_VAR: usize = {old};
    const REL_VAR: usize = {new};
    const RELATION_COMP_INDEX: usize = {comp};
    const REL_VAR_COMPONENTS: ::std::ops::Range<usize> = {start}..{end};
    let row = a_rows[CURRENT_VAR].0;
    let col = col_indexes[RELATION_COMP_INDEX];
    let arch = &a_refs[CURRENT_VAR];
    debug_assert_eq!(
        arch.columns[col].element_size(),
        size_of::<RelationVec>()
    );
    let ptr = unsafe {{ arch.columns[col].get(row) }} as *const RelationVec;
    let rel_vec = unsafe {{ &*ptr }};
    debug_assert!(rel_vec.len() > 0);
    if rel_index_{step} >= rel_vec.len() {{
        rel_index_{step} = 0;
        current_step -= 1;
    }} else {{
        // get aid/row for entity in relation
        let id = EntityId(rel_vec[rel_index_{step} as usize]);
        let (aid, arow) = bk.entities.get_archetype_unchecked(id);
        rel_index_{step} += 1;

        // if in target archetype set => go to next step
        if archetype_id_sets[REL_VAR].contains(&aid) {{
            let a_ref = &mut a_refs[REL_VAR];
            *a_ref = &bk.archetypes[aid.as_index()];
            a_ref.find_multiple_columns(
                &components_{new},
                &mut col_indexes[REL_VAR_COMPONENTS],
            );
            a_rows[REL_VAR] = arow;
"
        )
        .unwrap();

        if self.unequalities.is_empty() && self.rel_constraints.is_empty() {
            write!(
                append,
                "
            current_step += 1;"
            )
            .unwrap();
        } else {
            insert_checks(append, &self.unequalities, &self.rel_constraints);
            append.push_str(
                "
            {
                current_step -= 1;
            } else {
                current_step += 1;
            }",
            );
        }

        write!(
            append,
            "
        }}
    }}
}}
"
        )
        .unwrap();
        return step + 1;
    }
}

pub fn insert_checks(
    append: &mut String,
    unequalities: &[(isize, isize)],
    rel_constraints: &[(usize, isize, isize)],
) {
    append.push_str(
        r#"
            if"#,
    );

    let mut not_first = false;
    for (a, b) in unequalities {
        if not_first {
            write!(
                append,
                "
            ||"
            )
            .unwrap();
        }
        write!(
            append,
            "
                (::std::ptr::eq(a_refs[{a}], a_refs[{b}])
                 && a_rows[{a}] == a_rows[{b}])"
        )
        .unwrap();
        not_first = true;
    }
    for (rel_comp, a, b) in rel_constraints {
        if not_first {
            write!(
                append,
                "
            ||"
            )
            .unwrap();
        }
        write!(
            append,
            "
                {{
                    let arch = &a_refs[{a}];
                    let row = a_rows[{a}].0;
                    let col = col_indexes[{rel_comp}];
                    let rel_vec = unsafe {{
                        &*(arch.columns[col].get(row) as *const RelationVec)
                    }};
                    let check_ref = a_refs[{b}];
                    let to_check = check_ref.entities[a_rows[{b}].0 as usize];
                    !rel_vec.contains(&to_check.0)
                }}
                "
        )
        .unwrap();
        not_first = true;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn relation_join_unequality() {
        let gen = RelationJoin {
            relation_comp: 2,
            old: 0,
            new: 2,
            new_components: 3..5,
            unequalities: vec![(0, 2), (2, 1)],
            rel_constraints: vec![],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(3, &mut prepend, &mut append);
        assert_eq!(4, r);
        insta::assert_snapshot!(prepend, @"let mut rel_index_3 = 0;");
        insta::assert_snapshot!(append);
    }

    #[test]
    fn relation_join_constraint() {
        let gen = RelationJoin {
            relation_comp: 2,
            old: 0,
            new: 2,
            new_components: 3..5,
            unequalities: vec![],
            rel_constraints: vec![(5, 2, 1)],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(3, &mut prepend, &mut append);
        assert_eq!(4, r);
        insta::assert_snapshot!(prepend, @"let mut rel_index_3 = 0;");
        insta::assert_snapshot!(append);
    }
}
