use std::fmt::Debug;
use std::fmt::Write;
use std::ops::Range;

pub(crate) trait GeneratorNode: Debug {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String);
}

#[derive(Debug)]
pub(crate) struct RelationJoin {
    /// index of the component of `old` where the relation resides
    pub(crate) relation_comp: usize,
    pub(crate) old: isize,
    pub(crate) new: isize,
    pub(crate) new_components: Range<usize>,
    pub(crate) unequalities: Vec<(isize, isize)>,
}

impl GeneratorNode for RelationJoin {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String) {
        let old = self.old;
        let new = self.new;
        let comp = self.relation_comp;
        let uneqs = &self.unequalities;
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

        if uneqs.is_empty() {
            write!(
                append,
                "
            current_step += 1;"
            )
            .unwrap();
        } else {
            append.push_str(
                r#"
todo!("Complete this");
            if"#,
            );

            // TODO it depends also on join direction what current var etc is
            // so look up how I handled that above
            write!(
                append,
                "
                ::std::ptr::eq(a_refs[CURRENT_VAR], a_refs[REL_VAR])
                && a_rows[CURRENT_VAR] == a_rows[REL_VAR]"
            )
            .unwrap();
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
    }
}
