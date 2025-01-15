#![allow(dead_code)]
use std::collections::BTreeMap;
// TODO remove once finished
use std::fmt::Debug;
use std::fmt::Write;
use std::{collections::HashMap, ops::Range};

use crate::{Accessor, Component, Relation};
// TODO use write! instead of format! to save on intermediate allocations

#[derive(Clone)]
pub struct VarInfo {
    /// Index of this variable
    index: isize,
    /// relation type + other var index => index for relation component
    related_with: HashMap<(String, isize), usize>,
    /// indexes in component array
    component_range: Range<usize>,
    /// map from type to component index for accessors
    components: HashMap<String, usize>,
}

impl Debug for VarInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // fix ordering, for snapshot testing
        let related_with = self.related_with.iter().collect::<BTreeMap<_, _>>();
        let components = self.components.iter().collect::<BTreeMap<_, _>>();
        f.debug_struct("VarInfo")
            .field("index", &self.index)
            .field("related_with", &related_with)
            .field("component_range", &self.component_range)
            .field("components", &components)
            .finish()
    }
}

pub(crate) fn generate_archetype_sets(
    result: &mut String,
    vars: &[isize],
    prefills: &HashMap<isize, String>,
    components: &[Component],
    relations: &[Relation],
    uncomponents: &[Component],
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
            components: HashMap::new(),
        };
        result.push_str(&format!("let components_{var} = ["));
        // component
        for (ty, _) in components.iter().filter(|(_, id)| id == var) {
            result.push_str(&format!("\n    world.get_component_id::<{ty}>(),"));
            info.components.insert(ty.clone(), index);
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

    if uncomponents.is_empty() {
        result.push_str("let archetype_id_sets = [\n");
        for var in vars {
            result.push_str(&format!(
                "    bk.matching_archetypes(&components_{var}, &[]),\n"
            ));
        }
        result.push_str("];\n\n");
    } else {
        for var in vars {
            result.push_str(&format!("let uncomponents_{var} = ["));
            // component
            for (ty, _) in uncomponents.iter().filter(|(_, id)| id == var) {
                result.push_str(&format!("\n    world.get_component_id::<{ty}>(),"));
            }
            result.push_str("\n];\n\n");
        }
        result.push_str("let archetype_id_sets = [\n");
        for var in vars {
            result.push_str(&format!(
                "    bk.matching_archetypes(&components_{var}, &uncomponents_{var}),\n"
            ));
        }
        result.push_str("];\n\n");
    }
    return infos;
}

pub(crate) fn generate_fsm_context(
    result: &mut String,
    vars: &[isize],
    components: &[Component],
    relations: &[Relation],
) {
    let var_count = vars.len();
    let col_count = components.len() + relations.len() * 2;
    // TODO unify into struct to save on generated lines
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

pub(crate) fn generate_resumable_query_closure(
    result: &mut String,
    vars: &[isize],
    infos: &[VarInfo],
    relations: &[Relation],
    accessors: &[Accessor],
) {
    assert_eq!(infos.len(), vars.len());
    let prepend = result;
    let mut append = String::new();
    let (first, join_order) = compute_join_order(relations, infos);

    // TODO save on constants by directly applying the var
    append.push_str(
        "
::std::iter::from_fn(move || { loop { match current_step {",
    );

    // select first archetype
    {
        let first_info = &infos[first as usize];
        let Range { start, end } = &first_info.component_range;
        append.push_str(&format!(
            "
0 => {{
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
    a_max_rows[CURRENT_VAR] = a_ref.entities.len() as u32;
    current_step += 1;
}}
"
        ));
        // get row from first archetype
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
                &components_{new},
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
                todo!("RelationConstraints");
            }
        }
    }

    // yield row
    count += 1;
    write!(
        &mut append,
        "
// yield row
{count} => {{
    current_step -= 1;
    return Some(unsafe {{
        ("
    )
    .unwrap();
    for accessor in accessors {
        // TODO access mut
        match accessor {
            Accessor::Component(ty, var) => {
                let col = infos[*var as usize].components[ty];
                write!(
                    &mut append,
                    "
            (&*((&a_refs[{var}].columns[col_indexes[{col}]]).get(a_rows[{var}].0)
                as *const RefCell<{ty}>))
                .borrow(),"
                )
                .unwrap();
            }
            Accessor::ComponentMut(ty, var) => {
                let col = infos[*var as usize].components[ty];
                write!(
                    &mut append,
                    "
            (&*((&a_refs[{var}].columns[col_indexes[{col}]]).get(a_rows[{var}].0)
                as *const RefCell<{ty}>))
                .borrow_mut(),"
                )
                .unwrap();
            }
            Accessor::OutVar(var) => {
                // TODO lookup archetype and row and just return the entity from there lol
                write!(
                    &mut append,
                    "
            EntityViewDeferred::from_id_unchecked(world,
                                a_refs[{var}].entities[a_rows[{var}].0 as usize]),"
                )
                .unwrap();
            }
        }
    }
    write!(
        &mut append,
        "
        )
    }});
}}
"
    )
    .unwrap();

    // close the scope
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
    use crate::Accessor;

    #[test]
    fn test_generate_archetype_id_sets_relation() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let uncomponents = vec![("Bird".into(), 0), ("Fish".into(), 0), ("Bird".into(), 1)];
        let vars = vec![0, 1];
        let mut result = String::new();
        let prefills = HashMap::new();
        let infos;
        insta::assert_snapshot!({
            infos = generate_archetype_sets(&mut result, &vars, &prefills, &components, &relations,
                                            &uncomponents);
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

        let uncomponents_0 = [
            world.get_component_id::<Bird>(),
            world.get_component_id::<Fish>(),
        ];

        let uncomponents_1 = [
            world.get_component_id::<Bird>(),
        ];

        let archetype_id_sets = [
            bk.matching_archetypes(&components_0, &uncomponents_0),
            bk.matching_archetypes(&components_1, &uncomponents_1),
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
                components: {
                    "Health": 1,
                    "Unit": 0,
                },
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
                components: {
                    "Unit": 3,
                },
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
        let uncomponents = vec![];
        let relations = [];
        let vars = vec![0];
        let prefills = HashMap::new();
        let mut result = String::new();
        insta::assert_snapshot!({
            generate_archetype_sets(&mut result, &vars, &prefills, &components, &relations, &uncomponents);
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
        let uncomponents = vec![];
        let relations = vec![("Attack".into(), 1, 0)];
        let accessors = vec![
            Accessor::OutVar(0),
            Accessor::Component("Unit".to_string(), 0),
            Accessor::Component("Unit".to_string(), 1),
            Accessor::ComponentMut("Health".to_string(), 0),
        ];
        let vars = vec![0, 1];
        let mut result = String::new();
        let prefills = HashMap::new();
        let infos = generate_archetype_sets(
            &mut result,
            &vars,
            &prefills,
            &components,
            &relations,
            &uncomponents,
        );
        generate_fsm_context(&mut result, &vars, &components, &relations);
        insta::assert_snapshot!({
            generate_resumable_query_closure(&mut result, &vars, &infos, &relations, &accessors);
            result
        });
    }
}
