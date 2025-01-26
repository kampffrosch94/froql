use super::{
    relation_join::{insert_checks, insert_optional_comps},
    GeneratorNode,
};
use std::{fmt::Write, ops::Range};

#[derive(Debug)]
pub struct InvarInfo {
    pub var_index: isize,
    pub component_range: Range<usize>,
    /// type, index part of context variable name
    pub opt_components: Vec<(String, usize)>,
}

#[derive(Debug)]
pub struct InvarStart {
    /// index of the component of `old` where the relation resides
    pub unequalities: Vec<(isize, isize)>,
    pub rel_constraints: Vec<(usize, isize, isize)>,
    pub invars: Vec<InvarInfo>,
}

impl GeneratorNode for InvarStart {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String) -> usize {
        self.generate_invar_archetype_fill(prepend);

        if self.unequalities.is_empty() && self.rel_constraints.is_empty() {
            assert_eq!(step, 0);
            write!(
                append,
                r#"
{step} => {{
    return None;
}}
"#
            )
            .unwrap();
            // because 0 is our exit we have to start at 1
            prepend.push_str(
                "
current_step = 1;",
            );
            return step + 1;
        } else {
            write!(
                append,
                r#"
{step} => {{
"#
            )
            .unwrap();
            insert_checks(append, &self.unequalities, &self.rel_constraints);
            append.push_str(
                "
    {
        return None;
    } else {
        current_step += 2;
    }
}",
            );
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
}

impl InvarStart {
    fn generate_invar_archetype_fill(&self, prepend: &mut String) {
        // TODO is this needed?
        let mut append = String::new();
        for invar in &self.invars {
            let var_index = invar.var_index;
            let Range { start, end } = &invar.component_range;
            write!(
                &mut append,
                "
{{
    let (aid, arow) = bk.entities.get_archetype(invar_{var_index});
    let a_ref = &mut a_refs[{var_index}];
    *a_ref = &bk.archetypes[aid.as_index()];
    a_ref.find_multiple_columns(&components_{var_index}, &mut col_indexes[{start}..{end}]);
    a_rows[{var_index}] = arow;"
            )
            .unwrap();
            insert_optional_comps(prepend, &mut append, &invar.opt_components);
            write!(
                &mut append,
                "
}}
"
            )
            .unwrap();
        }
        prepend.push_str(&append);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn invar_trivial() {
        let gen = InvarStart {
            unequalities: vec![],
            rel_constraints: vec![],
            invars: vec![InvarInfo {
                var_index: 0,
                component_range: 0..2,
                opt_components: vec![],
            }],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(0, &mut prepend, &mut append);
        assert_eq!(1, r);
        insta::assert_snapshot!(prepend, @r#"
        {
            let (aid, arow) = bk.entities.get_archetype(invar_0);
            let a_ref = &mut a_refs[0];
            *a_ref = &bk.archetypes[aid.as_index()];
            a_ref.find_multiple_columns(&components_0, &mut col_indexes[0..2]);
            a_rows[0] = arow;
        }

        current_step = 1;
        "#);
        insta::assert_snapshot!(append, @r#"
        0 => {
            return None;
        }
        "#);
    }

    #[test]
    fn invar_unequality() {
        let gen = InvarStart {
            unequalities: vec![(0, 2), (2, 1)],
            rel_constraints: vec![],
            invars: vec![
                InvarInfo {
                    var_index: 0,
                    component_range: 0..2,
                    opt_components: vec![],
                },
                InvarInfo {
                    var_index: 1,
                    component_range: 3..5,
                    opt_components: vec![],
                },
                InvarInfo {
                    var_index: 2,
                    component_range: 5..7,
                    opt_components: vec![],
                },
            ],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(0, &mut prepend, &mut append);
        assert_eq!(2, r);
        insta::assert_snapshot!(prepend, @r#"
        {
            let (aid, arow) = bk.entities.get_archetype(invar_0);
            let a_ref = &mut a_refs[0];
            *a_ref = &bk.archetypes[aid.as_index()];
            a_ref.find_multiple_columns(&components_0, &mut col_indexes[0..2]);
            a_rows[0] = arow;
        }

        {
            let (aid, arow) = bk.entities.get_archetype(invar_1);
            let a_ref = &mut a_refs[1];
            *a_ref = &bk.archetypes[aid.as_index()];
            a_ref.find_multiple_columns(&components_1, &mut col_indexes[3..5]);
            a_rows[1] = arow;
        }

        {
            let (aid, arow) = bk.entities.get_archetype(invar_2);
            let a_ref = &mut a_refs[2];
            *a_ref = &bk.archetypes[aid.as_index()];
            a_ref.find_multiple_columns(&components_2, &mut col_indexes[5..7]);
            a_rows[2] = arow;
        }
        "#);
        insta::assert_snapshot!(append, @r#"
        0 => {

                    if
                        (::std::ptr::eq(a_refs[0], a_refs[2])
                         && a_rows[0] == a_rows[2])
                    ||
                        (::std::ptr::eq(a_refs[2], a_refs[1])
                         && a_rows[2] == a_rows[1])
            {
                return None;
            } else {
                current_step += 2;
            }
        }
        1 => {
            return None;
        }
        "#);
    }
}
