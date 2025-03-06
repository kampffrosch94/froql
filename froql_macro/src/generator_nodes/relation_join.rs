use super::relation_helper::{
    relation_helpers_init_and_set_col, relation_helpers_set_rows, RelationHelperInfo,
    UnrelationHelperInfo,
};
use super::types::{RelationConstraint, UnrelationConstraint};
use super::GeneratorNode;
use std::fmt::Write;
use std::ops::Range;

#[derive(Debug)]
pub struct RelationJoin {
    /// index of the component of `old` where the relation resides
    pub new: isize,
    pub new_components: Range<usize>,
    pub unequal_constraints: Vec<(isize, isize)>,
    /// RelationHelpers that constrain the new var
    pub rel_constraints: Vec<RelationConstraint>,
    /// UnrelationHelpers that constrain the new var
    pub unrel_constraints: Vec<UnrelationConstraint>,
    pub opt_components: Vec<(String, usize)>,
    /// RelationHelpers that depend on the new var
    pub new_relation_helpers: Vec<RelationHelperInfo>,
    /// UnrelationHelpers that depend on the new var
    pub new_unrelation_helpers: Vec<UnrelationHelperInfo>,
    /// the relationhelper that contains the Relation(old, new)
    pub new_helper_nr: usize,
}

impl GeneratorNode for RelationJoin {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String) -> usize {
        let new = self.new;
        let Range { start, end } = &self.new_components;
        let helper_nr = self.new_helper_nr;
        write!(
            append,
            "
// follow relation
{step} => {{
    const REL_VAR: usize = {new};
    const REL_VAR_COMPONENTS: ::std::ops::Range<usize> = {start}..{end};
    if let Some(id) = rel_helper_{helper_nr}.next_related() {{
        // get aid/row for entity in relation
        let (aid, arow) = bk.entities.get_archetype_unchecked(id);

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

        // check constraints if there are any
        if self.unequal_constraints.is_empty()
            && self.rel_constraints.is_empty()
            && self.unrel_constraints.is_empty()
        {
            insert_optional_comps(prepend, append, &self.opt_components);
            relation_helpers_init_and_set_col(
                prepend,
                append,
                &self.new_relation_helpers,
                &self.new_unrelation_helpers,
            );
            relation_helpers_set_rows(
                append,
                &self.new_relation_helpers,
                &self.new_unrelation_helpers,
            );
            write!(
                append,
                "
            current_step += 1;"
            )
            .unwrap();
        } else {
            insert_checks(
                append,
                &self.unequal_constraints,
                &self.rel_constraints,
                &self.unrel_constraints,
            );
            append.push_str(
                "
            {
            } else {",
            );
            // handle optional components only in positive branch
            insert_optional_comps(prepend, append, &self.opt_components);
            relation_helpers_init_and_set_col(
                prepend,
                append,
                &self.new_relation_helpers,
                &self.new_unrelation_helpers,
            );
            relation_helpers_set_rows(
                append,
                &self.new_relation_helpers,
                &self.new_unrelation_helpers,
            );
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
    }} else {{
        current_step -= 1;
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
    rel_constraints: &[RelationConstraint],
    unrel_constraints: &[UnrelationConstraint],
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
    for rc in rel_constraints {
        let helper_nr = &rc.helper_nr;
        let id = &rc
            .checked_invar
            .map(|it| format!("invar_{it}.id"))
            .unwrap_or_else(|| "id".to_string());
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
                !rel_helper_{helper_nr}.has_relation({id})
                "
        )
        .unwrap();
        not_first = true;
    }
    for rc in unrel_constraints {
        let helper_nr = &rc.helper_nr;
        let id = &rc
            .checked_invar
            .map(|it| format!("invar_{it}.id"))
            .unwrap_or_else(|| "id".to_string());
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
                !unrel_helper_{helper_nr}.satisfied({id})
                "
        )
        .unwrap();
        not_first = true;
    }
}

pub fn insert_optional_comps(
    prepend: &mut String,
    append: &mut String,
    opt_components: &[(String, usize)],
) {
    for (ty, id) in opt_components {
        write!(
            prepend,
            r#"
let opt_cid_{id} = world.get_component_id::<{ty}>();
let mut opt_col_{id} = None;"#
        )
        .unwrap();
        write!(
            append,
            r#"
            opt_col_{id} = a_ref.find_column_opt(opt_cid_{id});
"#
        )
        .unwrap();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn relation_join_unequality() {
        let generator = RelationJoin {
            new: 2,
            new_components: 3..5,
            unequal_constraints: vec![(0, 2), (2, 1)],
            rel_constraints: vec![],
            unrel_constraints: vec![],
            opt_components: vec![],
            new_relation_helpers: vec![],
            new_unrelation_helpers: vec![],
            new_helper_nr: 0,
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = generator.generate(3, &mut prepend, &mut append);
        assert_eq!(4, r);
        insta::assert_snapshot!(prepend, @"");
        insta::assert_snapshot!(append);
    }

    #[test]
    fn relation_join_constraint() {
        let generator = RelationJoin {
            new: 2,
            new_components: 3..5,
            unequal_constraints: vec![],
            rel_constraints: vec![RelationConstraint {
                helper_nr: 0,
                checked_invar: None,
            }],
            unrel_constraints: vec![],
            opt_components: vec![],
            new_relation_helpers: vec![],
            new_unrelation_helpers: vec![],
            new_helper_nr: 0,
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = generator.generate(3, &mut prepend, &mut append);
        assert_eq!(4, r);
        insta::assert_snapshot!(prepend, @"");
        insta::assert_snapshot!(append);
    }

    #[test]
    fn relation_join_optional() {
        let generator = RelationJoin {
            new: 2,
            new_components: 3..5,
            unequal_constraints: vec![],
            rel_constraints: vec![],
            unrel_constraints: vec![],
            opt_components: vec![("OptA".into(), 0), ("OptB".into(), 1)],
            new_relation_helpers: vec![],
            new_unrelation_helpers: vec![],
            new_helper_nr: 0,
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = generator.generate(3, &mut prepend, &mut append);
        assert_eq!(4, r);
        insta::assert_snapshot!(prepend, @r#"
        let opt_cid_0 = world.get_component_id::<OptA>();
        let mut opt_col_0 = None;
        let opt_cid_1 = world.get_component_id::<OptB>();
        let mut opt_col_1 = None;
        "#);
        insta::assert_snapshot!(append);
    }
}
