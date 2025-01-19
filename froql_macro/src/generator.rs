#![allow(dead_code)]
use std::collections::BTreeMap;
// TODO remove once finished
use std::fmt::Debug;
use std::fmt::Write;
use std::{collections::HashMap, ops::Range};

use crate::generator_nodes::invar_start::InvarStart;
use crate::generator_nodes::relation_join::RelationJoin;
use crate::generator_nodes::GeneratorNode;
use crate::ANYVAR;
use crate::{Accessor, Component, Relation};
// TODO use write! instead of format! to save on intermediate allocations

#[derive(Clone, PartialEq, Eq)]
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

impl PartialOrd for VarInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        return self.index.partial_cmp(&other.index);
    }
}

impl Ord for VarInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        return self.index.cmp(&other.index);
    }
}

pub(crate) fn generate_invar_captures(result: &mut String, prefills: &HashMap<isize, String>) {
    // vec + sort for determinism
    let mut v = prefills.iter().collect::<Vec<_>>();
    v.sort();

    for (index, name) in v {
        write!(
            result,
            "let invar_{index}: Entity = {name};
"
        )
        .unwrap();
    }
}

pub(crate) fn generate_invar_archetype_fill(
    result: &mut String,
    infos: &[VarInfo],
    prefills: &HashMap<isize, String>,
) {
    for info in infos.iter().filter(|it| prefills.contains_key(&it.index)) {
        let var_index = info.index;
        let Range { start, end } = &info.component_range;
        write!(
            result,
            "
{{
    let (aid, arow) = bk.entities.get_archetype(invar_{var_index});
    let a_ref = &mut a_refs[{var_index}];
    *a_ref = &bk.archetypes[aid.as_index()];
    a_ref.find_multiple_columns(&components_{var_index}, &mut col_indexes[{start}..{end}]);
    a_rows[{var_index}] = arow;
}}
"
        )
        .unwrap();
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
    assert_ne!(
        0,
        components.len() + relations.len(),
        "A query needs have at least one Component or Relation."
    );
    assert_ne!(
        0,
        vars.len(),
        "A query needs to have at least one Variable."
    );

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
            if prefills.contains_key(var) {
                // don't need this for prefills
                result.push_str(&format!("    Vec::new(),\n"));
            } else {
                write!(
                    result,
                    "    bk.matching_archetypes(&components_{var}, &[]),\n"
                )
                .unwrap();
            }
        }
        result.push_str("];\n\n");
    } else {
        for var in vars {
            if prefills.contains_key(var) {
                continue;
            }

            result.push_str(&format!("let uncomponents_{var} = ["));
            // component
            for (ty, _) in uncomponents.iter().filter(|(_, id)| id == var) {
                result.push_str(&format!("\n    world.get_component_id::<{ty}>(),"));
            }
            result.push_str("\n];\n\n");
        }
        result.push_str("let archetype_id_sets = [\n");
        for var in vars {
            if prefills.contains_key(var) {
                // don't need this for prefills
                result.push_str(&format!("    Vec::new(),\n"));
            } else {
                write!(
                    result,
                    "    bk.matching_archetypes(&components_{var}, &uncomponents_{var}),\n"
                )
                .unwrap();
            }
        }
        result.push_str("];\n\n");
    }
    return infos;
}

pub(crate) fn generate_fsm_context(
    result: &mut String,
    vars: &[isize],
    prefills: &HashMap<isize, String>,
    components: &[Component],
    relations: &[Relation],
) {
    let var_count = vars.len();
    let col_count = components.len() + relations.len() * 2;
    let start_step = if prefills.is_empty() { 0 } else { 1 };
    result.push_str(&format!(
        "
// result set
const VAR_COUNT: usize = {var_count};
let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];
let mut a_rows = [ArchetypeRow(u32::MAX); VAR_COUNT];

// general context for statemachine
let mut current_step = {start_step};
let mut a_max_rows = [0; VAR_COUNT];
let mut a_next_indexes = [usize::MAX; VAR_COUNT];
let mut col_indexes = [usize::MAX; {col_count}];
"
    ));
}

pub(crate) fn generate_resumable_query_closure(
    result: &mut String,
    vars: &[isize],
    prefills: &HashMap<isize, String>,
    infos: &[VarInfo],
    relations: &[Relation],
    unequals: &[(isize, isize)],
    accessors: &[Accessor],
) {
    assert_eq!(infos.len(), vars.len());
    let prepend = result;
    let mut append = String::new();
    let (first, invar_unequals, join_order) =
        compute_join_order(relations, infos, prefills, unequals);

    // TODO save on constants by directly applying the var
    append.push_str(
        "
::std::iter::from_fn(move || { loop { match current_step {",
    );

    let mut step_count;

    if prefills.is_empty() {
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
        step_count = 2;
    } else {
        step_count = InvarStart {
            unequalities: invar_unequals,
        }
        .generate(0, prepend, &mut append);
    }
    // follow relations/constraints
    for step in join_order {
        match step {
            JoinKind::NewJoin(relation_comp, old, new, unequalities) => {
                step_count = RelationJoin {
                    relation_comp,
                    old,
                    new,
                    new_components: infos[new as usize].component_range.clone(),
                    unequalities,
                }
                .generate(step_count, prepend, &mut append);
            }
            JoinKind::RelationConstraint(_, _, _) => {
                todo!("RelationConstraints");
            }
        }
    }

    // yield row
    write!(
        &mut append,
        "
// yield row
{step_count} => {{
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
    /// component id, old var, new var, unequals to check
    NewJoin(usize, isize, isize, Vec<(isize, isize)>),
    /// component id, old var, new var
    RelationConstraint(usize, isize, isize),
    // /// var a, var b
    // Unequals(isize, isize),
}

fn compute_join_order(
    relations: &[Relation],
    infos: &[VarInfo],
    prefills: &HashMap<isize, String>,
    unequals: &[(isize, isize)],
) -> (isize, Vec<(isize, isize)>, Vec<JoinKind>) {
    let mut result: Vec<JoinKind> = Vec::new();
    let mut available: Vec<isize> = Vec::new();
    let mut unequals = Vec::from(unequals);
    let mut work_left: Vec<Relation> = relations
        .iter()
        .cloned()
        // anyvars only matter as components for constraining archetype sets
        .filter(|(_, from, to)| *from != ANYVAR && *to != ANYVAR)
        .collect();

    // figure out what to start with
    if prefills.is_empty() {
        // I think its a decent metric to use the most constrained variable first
        let first = infos
            .iter()
            .max_by_key(|it| it.component_range.len())
            .unwrap();
        available.push(first.index);
    } else {
        for (var, _) in prefills {
            available.push(*var);
            available.sort();
        }
    }

    // using a closure so I don't have to duplicate this part
    let mut newly_available_unequals = |available: &mut Vec<isize>| {
        let mut result = Vec::new();
        while let Some(index) = unequals
            .iter()
            .position(|(a, b)| available.contains(a) && available.contains(b))
        {
            let (a, b) = unequals[index];
            result.push((a, b));
            unequals.swap_remove(index);
        }
        result
    };
    let invar_unequals = newly_available_unequals(&mut available);

    // compute join
    for _ in 0..work_left.len() {
        // find next viable for joining and remove it from working list
        // always handle constraints first, because it may let us skip work
        let next_constraint = {
            let pos = work_left
                .iter()
                .position(|(_, from, to)| available.contains(from) && available.contains(to));
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
                        .any(|avail| *avail == rel.1 || *avail == rel.2)
                });
                pos.map(|pos| work_left.remove(pos))
            };
            if let Some(join) = next_join {
                let reversed = available.iter().any(|avail| *avail == join.2);
                let old = if reversed { join.2 } else { join.1 };
                let new = if reversed { join.1 } else { join.2 };
                let info = &infos[old as usize];
                assert_eq!(old, info.index);
                let comp_index = info.related_with[&(join.0, new)];
                available.push(new);
                let uneqs = newly_available_unequals(&mut available);
                result.push(JoinKind::NewJoin(comp_index, old, new, uneqs));
            } else {
                panic!("Cross joins are not supported.")
            }
        }
    }
    assert!(unequals.is_empty());
    return (available[0], invar_unequals, result);
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

        let join_order = compute_join_order(&relations, &infos, &prefills, &[]);
        insta::assert_debug_snapshot!(join_order, @r#"
        (
            0,
            [],
            [
                NewJoin(
                    2,
                    0,
                    1,
                    [],
                ),
            ],
        )
        "#);

        let unequals = vec![(0, 1)];
        let join_order = compute_join_order(&relations, &infos, &prefills, &unequals);
        insta::assert_debug_snapshot!(join_order, @r#"
        (
            0,
            [],
            [
                NewJoin(
                    2,
                    0,
                    1,
                    [
                        (
                            0,
                            1,
                        ),
                    ],
                ),
            ],
        )
        "#);
    }

    #[test]
    fn test_generate_invar_captures() {
        let mut result = String::new();
        let mut prefills = HashMap::new();
        prefills.insert(1, "player".to_string());
        prefills.insert(2, "somebody".to_string());
        insta::assert_snapshot!({
            generate_invar_captures(&mut result, &prefills);
            result
        }, @r#"
        let invar_1: Entity = player;
        let invar_2: Entity = somebody;
        "#);

        let mut result = String::new();
        let mut prefills = HashMap::new();
        prefills.insert(1, "player".to_string());
        insta::assert_snapshot!({
            generate_invar_captures(&mut result, &prefills);
            result
        }, @"let invar_1: Entity = player;");

        // empty
        let mut result = String::new();
        let prefills = HashMap::new();
        assert_eq!(
            {
                generate_invar_captures(&mut result, &prefills);
                result
            },
            ""
        );
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
        let prefills = HashMap::new();

        insta::assert_snapshot!({
            generate_fsm_context(&mut result, &vars, &prefills, &components, &relations);
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
        generate_fsm_context(&mut result, &vars, &prefills, &components, &relations);
        generate_invar_archetype_fill(&mut result, &infos, &prefills);
        generate_resumable_query_closure(
            &mut result,
            &vars,
            &prefills,
            &infos,
            &relations,
            &[], //unequals
            &accessors,
        );
        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_invar() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let uncomponents = vec![];
        let relations = vec![("Attack".into(), 1, 0)];
        let accessors = vec![
            Accessor::Component("Unit".to_string(), 0),
            Accessor::Component("Unit".to_string(), 1),
            Accessor::ComponentMut("Health".to_string(), 0),
        ];
        let vars = vec![0, 1];
        let mut prefills = HashMap::new();
        prefills.insert(1, "player".to_string());

        let mut result = String::new();
        generate_invar_captures(&mut result, &prefills);
        let infos = generate_archetype_sets(
            &mut result,
            &vars,
            &prefills,
            &components,
            &relations,
            &uncomponents,
        );
        dbg!(&infos);
        generate_fsm_context(&mut result, &vars, &prefills, &components, &relations);
        generate_invar_archetype_fill(&mut result, &infos, &prefills);

        generate_resumable_query_closure(
            &mut result,
            &vars,
            &prefills,
            &infos,
            &relations,
            &[],
            &accessors,
        );

        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_unequals() {
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
        generate_fsm_context(&mut result, &vars, &prefills, &components, &relations);
        generate_invar_archetype_fill(&mut result, &infos, &prefills);
        let unequals = vec![(0, 1)];
        generate_resumable_query_closure(
            &mut result,
            &vars,
            &prefills,
            &infos,
            &relations,
            &unequals,
            &accessors,
        );
        insta::assert_snapshot!(result);
    }
}
