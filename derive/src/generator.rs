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
    result.push_str(
        "
// context for this specific statemachine
let mut rel_index_2 = 0; <================= TODO

std::iter::fstd::iter::from_fn(move || { loop { match current_step {
    _ => unreachable!(),
}}})
",
    );
}

#[derive(Debug)]
enum JoinKind {
    /// component id, old var, new var
    NewJoin(usize, isize, isize),
    /// component id, old var, new var
    RelationConstraint(usize, isize, isize),
}

pub fn compute_join_order(relations: &[Relation], infos: &[VarInfo]) -> (isize, Vec<JoinKind>) {
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
                let old = if reversed {join.2} else {join.1};
                let new = if reversed {join.1} else {join.2};
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
            generate_resumable_query_closure(&mut result, &vars,
                                              &infos,&components, &relations);
            result
        }, @r#"
        // context for this specific statemachine
        let mut rel_index_2 = 0; <================= TODO

        std::iter::fstd::iter::from_fn(move || { loop { match current_step {
            _ => unreachable!(),
        }}})
        "#);
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
