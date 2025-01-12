#![allow(dead_code)] // TODO remove once finished
use std::{collections::HashMap, ops::Range};

use crate::{Component, Relation};

#[derive(Debug, Clone)]
pub struct VarInfo {
    /// Index of this variable
    index: isize,
    /// other var index, index for relation component
    related_with: HashMap<(String, isize), usize>,
    /// indexes in component array
    component_range: Range<usize>,
}

fn generate_archetype_sets(
    result: &mut String,
    vars: &[isize],
    components: &[Component],
    relations: &[Relation],
) -> Vec<VarInfo> {
    assert_ne!(0, components.len() + relations.len());
    assert_ne!(0, vars.len());

    let mut infos = Vec::new();
    let mut index = 0;

    for var in vars {
        let mut info = VarInfo {
            index: *var,
            related_with: HashMap::new(),
            component_range: index..index,
        };
        result.push_str(&format!("let components_{var} = ["));
        for (ty, _) in components.iter().filter(|(_, id)| id == var) {
            result.push_str(&format!("\n    world.get_component_id::<{ty}>(),"));
            index += 1;
            info.component_range.end += 1;
        }
        // relation from
        for (ty, _, other) in relations.iter().filter(|(_, id, _)| id == var) {
            result.push_str(&format!(
                "\n    bk.get_component_id_unchecked(TypeId::of::<Relation<{ty}>>()),"
            ));
            info.related_with.insert((ty.clone(), *other), index);
            index += 1;
            info.component_range.end += 1;
        }
        // relation to
        for (ty, other, _) in relations.iter().filter(|(_, _, id)| id == var) {
            result.push_str("\n    ");
            result.push_str(&format!(
                "bk.get_component_id_unchecked(TypeId::of::<Relation<{ty}>>()).flip_target(),"
            ));
            info.related_with.insert((ty.clone(), *other), index);
            index += 1;
            info.component_range.end += 1;
        }
        result.push_str("\n];\n\n");
        infos.push(info);
    }

    result.push_str("let archetype_id_sets = [\n");
    for var in vars {
        result.push_str(&format!(
            "    bk.matching_archetypes(&components_{var}, &[]),\n"
        ));
    }
    result.push_str("];\n\n");
    return infos;
}

fn generate_fsm_context(
    result: &mut String,
    vars: &[isize],
    components: &[Component],
    relations: &[Relation],
) {
    let var_count = vars.len();
    let col_count = components.len() + relations.len() * 2;
    result.push_str(&format!(
        "
// result set
const VAR_COUNT: usize = {var_count};
let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];
let mut a_rows = [ArchetypeRow(u32::MAX); VAR_COUNT];

// general context for statemachine
let mut current_step = 0;
let mut a_max_rows = [0; VAR_COUNT];
let mut a_next_indexes = [usize::MAX; VAR_COUNT];
let mut col_indexes = [usize::MAX; {col_count}];
"
    ));
}

fn generate_resumable_query_closure(
    result: &mut String,
    vars: &[isize],
    infos: &[VarInfo],
    components: &[Component],
    relations: &[Relation],
) {
    let prepend = result;
    let mut append = String::new();
    let (first, join_order) = compute_join_order(relations, infos);

    append.push_str(
        "
std::iter::fstd::iter::from_fn(move || { loop { match current_step {",
    );

    // select first archetype
    {
        let first_info = &infos[first as usize];
        let Range { start, end } = &first_info.component_range;
        append.push_str(&format!(
            "
0 => {{
    const CURRENT_VAR: usize = {first};
    const CURRENT_VAR_COMPONENTS: Range<usize> = {start}..{end};
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
        &components_me,
        &mut col_indexes[CURRENT_VAR_COMPONENTS],
    );
    a_max_rows[CURRENT_VAR] = a_ref.entities.len() as u32;
    current_step += 1;
}}
"
        ));
        // TODO get row from first
        append.push_str(&format!(
            "
// next row in archetype
1 => {{
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
        ));
    }
    let mut count = 1;
    // follow relations/constraints
    for step in join_order {
        count += 1;
        match step {
            JoinKind::NewJoin(comp, old, new) => {
                let new_info = &infos[new as usize];
                let Range { start, end } = &new_info.component_range;
                // TODO prepend state for current join
                prepend.push_str(&format!("\nlet mut rel_index_{count} = 0;"));
                append.push_str(&format!(
                    "
// follow relation
{count} => {{
    const CURRENT_VAR: usize = {old};
    const REL_VAR: usize = {new};
    const RELATION_COMP_INDEX: usize = {comp};
    const REL_VAR_COMPONENTS: Range<usize> = {start}..{end};
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
    if rel_index_{count} >= rel_vec.len() {{
        rel_index_{count} = 0;
        current_step -= 1;
    }} else {{
        // get aid/row for entity in relation
        let id = EntityId(rel_vec[rel_index_{count} as usize]);
        let (aid, arow) = bk.entities.get_archetype_unchecked(id);
        rel_index_{count} += 1;

        // if in target archetype set => go to next step
        if archetype_id_sets[REL_VAR].contains(&aid) {{
            let a_ref = &mut a_refs[REL_VAR];
            *a_ref = &bk.archetypes[aid.as_index()];
            a_ref.find_multiple_columns(
                &components_other,
                &mut col_indexes[REL_VAR_COMPONENTS],
            );
            a_rows[REL_VAR] = arow;

            current_step += 1;
        }}
    }}
}}
",
                ));
            }
            JoinKind::RelationConstraint(_, _, _) => {
                todo!();
            }
        }
    }
    // TODO yield row
    append.push_str(
        "
_ => unreachable!(),
}}})
",
    );
    prepend.push_str("\n");
    prepend.push_str(&append);
}

#[derive(Debug)]
enum JoinKind {
    /// component id, old var, new var
    NewJoin(usize, isize, isize),
    /// component id, old var, new var
    RelationConstraint(usize, isize, isize),
}

fn compute_join_order(relations: &[Relation], infos: &[VarInfo]) -> (isize, Vec<JoinKind>) {
    // TODO figure out the variable to start with
    // I think its a decent metric to use the most constrained variable first
    let first = infos
        .iter()
        .max_by_key(|it| it.component_range.len())
        .unwrap();

    let mut result: Vec<JoinKind> = Vec::new();
    let mut available: Vec<VarInfo> = vec![first.clone()];
    let mut work_left: Vec<Relation> = Vec::from(relations);
    // TODO compute join
    for _ in 0..work_left.len() {
        // find next viable for joining and remove it from working list
        let next_constraint = {
            let pos = work_left.iter().position(|rel| {
                available
                    .iter()
                    .any(|avail| avail.index == rel.1 && avail.index == rel.2)
            });
            pos.map(|pos| work_left.remove(pos))
        };
        if let Some(constraint) = next_constraint {
            let old = constraint.1;
            let new = constraint.2;
            let info = &infos[old as usize];
            assert_eq!(old, info.index);
            let comp_index = info.related_with[&(constraint.0, constraint.2)];
            result.push(JoinKind::RelationConstraint(comp_index, old, new));
        } else {
            let next_join = {
                let pos = work_left.iter().position(|rel| {
                    available
                        .iter()
                        .any(|avail| avail.index == rel.1 || avail.index == rel.2)
                });
                pos.map(|pos| work_left.remove(pos))
            };
            if let Some(join) = next_join {
                let reversed = available.iter().any(|it| it.index == join.2);
                let old = if reversed { join.2 } else { join.1 };
                let new = if reversed { join.1 } else { join.2 };
                let info = &infos[old as usize];
                assert_eq!(old, info.index);
                dbg!(info);
                let comp_index = info.related_with[&(join.0, new)];
                result.push(JoinKind::NewJoin(comp_index, old, new));
                available.push(infos[new as usize].clone());
            } else {
                panic!("Cross joins are not supported.")
            }
        }
    }
    return (first.index, result);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generate_archetype_id_sets_relation() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let vars = vec![0, 1];
        let mut result = String::new();
        let infos;
        insta::assert_snapshot!({
            infos = generate_archetype_sets(&mut result, &vars, &components, &relations);
            result
        }, @r#"
        let components_0 = [
            world.get_component_id::<Unit>(),
            world.get_component_id::<Health>(),
            bk.get_component_id_unchecked(TypeId::of::<Relation<Attack>>()).flip_target(),
        ];

        let components_1 = [
            world.get_component_id::<Unit>(),
            bk.get_component_id_unchecked(TypeId::of::<Relation<Attack>>()),
        ];

        let archetype_id_sets = [
            bk.matching_archetypes(&components_0, &[]),
            bk.matching_archetypes(&components_1, &[]),
        ];
        "#);

        insta::assert_debug_snapshot!(infos, @r#"
        [
            VarInfo {
                index: 0,
                related_with: {
                    (
                        "Attack",
                        1,
                    ): 2,
                },
                component_range: 0..3,
            },
            VarInfo {
                index: 1,
                related_with: {
                    (
                        "Attack",
                        0,
                    ): 4,
                },
                component_range: 3..5,
            },
        ]
        "#);

        let join_order = compute_join_order(&relations, &infos);
        insta::assert_debug_snapshot!(join_order, @r#"
        (
            0,
            [
                NewJoin(
                    2,
                    0,
                    1,
                ),
            ],
        )
        "#);
    }

    #[test]
    fn test_generate_archetype_id_sets_trivial() {
        let components = vec![("Pos".into(), 0), ("Speed".into(), 0)];
        let relations = [];
        let vars = vec![0];
        let mut result = String::new();
        insta::assert_snapshot!({
            generate_archetype_sets(&mut result, &vars, &components, &relations);
            result
        }, @r#"
        let components_0 = [
            world.get_component_id::<Pos>(),
            world.get_component_id::<Speed>(),
        ];

        let archetype_id_sets = [
            bk.matching_archetypes(&components_0, &[]),
        ];
        "#);
    }

    #[test]
    fn test_generate_result_set() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let vars = vec![0, 1];
        let mut result = String::new();

        insta::assert_snapshot!({
            generate_fsm_context(&mut result, &vars, &components, &relations);
            result
        }, @r#"
        // result set
        const VAR_COUNT: usize = 2;
        let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];
        let mut a_rows = [ArchetypeRow(u32::MAX); VAR_COUNT];

        // general context for statemachine
        let mut current_step = 0;
        let mut a_max_rows = [0; VAR_COUNT];
        let mut a_next_indexes = [usize::MAX; VAR_COUNT];
        let mut col_indexes = [usize::MAX; 5];
        "#);
    }

    #[test]
    fn test_generate_resumable_query_closure() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let vars = vec![0, 1];
        let mut result = String::new();
        let infos = generate_archetype_sets(&mut String::new(), &vars, &components, &relations);
        insta::assert_snapshot!({
            generate_resumable_query_closure(&mut result, &vars, &infos, &components, &relations);
            result
        });
    }

    #[test]
    fn test_compute_join_order() {
        let relations = vec![("Attack".into(), 1, 0)];
        let infos = vec![
            VarInfo {
                index: 0,
                related_with: HashMap::from([(("Attack".into(), 1), 2)]),
                component_range: 0..3,
            },
            VarInfo {
                index: 1,
                related_with: HashMap::from([(("Attack".into(), 0), 4)]),
                component_range: 4..5,
            },
        ];
        let joined = compute_join_order(&relations, &infos);
        insta::assert_debug_snapshot!(joined, @r#"
        (
            0,
            [
                NewJoin(
                    2,
                    0,
                    1,
                ),
            ],
        )
        "#);
    }
}
