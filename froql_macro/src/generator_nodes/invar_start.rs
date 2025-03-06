use super::{
    relation_helper::{
        relation_helpers_init_and_set_col, relation_helpers_set_rows, RelationHelperInfo,
        UnrelationHelperInfo,
    },
    relation_join::{insert_checks, insert_optional_comps},
    types::{RelationConstraint, UnrelationConstraint},
    GeneratorNode,
};
use std::{fmt::Write, ops::Range};

#[derive(Debug)]
pub struct InvarInfo {
    pub var_index: isize,
    pub component_range: Range<usize>,
    /// type, index part of context variable name
    pub opt_components: Vec<(String, usize)>,
    pub relation_helpers: Vec<RelationHelperInfo>,
    pub unrelation_helpers: Vec<UnrelationHelperInfo>,
}

#[derive(Debug)]
pub struct InvarStart {
    pub unequalities: Vec<(isize, isize)>,
    pub rel_constraints: Vec<RelationConstraint>,
    pub unrel_constraints: Vec<UnrelationConstraint>,
    pub invars: Vec<InvarInfo>,
}

impl GeneratorNode for InvarStart {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String) -> usize {
        write!(
            append,
            r#"
{step} => {{
"#
        )
        .unwrap();
        self.generate_invar_archetype_fill(prepend, append);

        if self.unequalities.is_empty()
            && self.rel_constraints.is_empty()
            && self.unrel_constraints.is_empty()
        {
            append.push_str(
                "
    current_step += 2;
}",
            );
        } else {
            insert_checks(
                append,
                &self.unequalities,
                &self.rel_constraints,
                &self.unrel_constraints,
            );
            append.push_str(
                "
    {
        return None;
    } else {
        current_step += 2;
    }
}",
            );
        }
        let next_step = step + 1;
        // end state
        write!(
            append,
            r#"
{next_step} => {{
    return None;
}}
"#
        )
        .unwrap();
        return step + 2;
    }
}

impl InvarStart {
    fn generate_invar_archetype_fill(&self, prepend: &mut String, append: &mut String) {
        // another string is needed because we prepend optional components before this block
        for invar in &self.invars {
            let var_index = invar.var_index;
            let Range { start, end } = &invar.component_range;
            write!(
                append,
                "
{{
    let (aid, arow) = bk.entities.get_archetype(invar_{var_index});
    let a_ref = &mut a_refs[{var_index}];
    *a_ref = &bk.archetypes[aid.as_index()];
    if !a_ref.find_multiple_columns_fallible(&components_{var_index}, &mut col_indexes[{start}..{end}]) {{
        return None;
    }}
    a_rows[{var_index}] = arow;"
            )
            .unwrap();

            insert_optional_comps(prepend, append, &invar.opt_components);
            relation_helpers_init_and_set_col(
                prepend,
                append,
                &invar.relation_helpers,
                &invar.unrelation_helpers,
            );
            relation_helpers_set_rows(append, &invar.relation_helpers, &invar.unrelation_helpers);

            write!(
                append,
                "
}}
"
            )
            .unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn invar_trivial() {
        let generator = InvarStart {
            unequalities: vec![],
            rel_constraints: vec![],
            unrel_constraints: vec![],
            invars: vec![InvarInfo {
                var_index: 0,
                component_range: 0..2,
                opt_components: vec![],
                relation_helpers: vec![],
                unrelation_helpers: vec![],
            }],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = generator.generate(0, &mut prepend, &mut append);
        assert_eq!(2, r);
        insta::assert_snapshot!(prepend, @"");
        insta::assert_snapshot!(append, @r#"
        0 => {

        {
            let (aid, arow) = bk.entities.get_archetype(invar_0);
            let a_ref = &mut a_refs[0];
            *a_ref = &bk.archetypes[aid.as_index()];
            if !a_ref.find_multiple_columns_fallible(&components_0, &mut col_indexes[0..2]) {
                return None;
            }
            a_rows[0] = arow;
        }

            current_step += 2;
        }
        1 => {
            return None;
        }
        "#);
    }
}
