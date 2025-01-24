#![allow(dead_code)]
use std::collections::BTreeMap;
// TODO remove once finished
use std::fmt::Debug;
use std::fmt::Write;
use std::{collections::HashMap, ops::Range};

use crate::generator_nodes::archetype_start::ArchetypeStart;
use crate::generator_nodes::invar_start::InvarStart;
use crate::generator_nodes::relation_join::insert_optional_comps;
use crate::generator_nodes::relation_join::RelationJoin;
use crate::generator_nodes::GeneratorNode;
use crate::ANYVAR;
use crate::{Accessor, Component, Relation};
// TODO use write! instead of format! to save on intermediate allocations

#[derive(Default, Debug)]
pub struct Generator {
    pub vars: Vec<isize>,
    pub prefills: HashMap<isize, String>,
    pub components: Vec<Component>,
    pub relations: Vec<Relation>,
    pub uncomponents: Vec<Component>,
    pub opt_components: Vec<(String, isize, usize)>,
    pub unequals: Vec<(isize, isize)>,
    pub accessors: Vec<Accessor>,
}

impl Generator {
    pub fn generate(&self, world: &str) -> String {
        let mut result = String::new();

        result.push_str("{");

        generate_invar_captures(&mut result, &self.prefills);

        write!(
            &mut result,
            "
let world: &World = &{world};
let bk = &world.bookkeeping;
"
        )
        .unwrap();

        let infos = generate_archetype_sets(
            &mut result,
            &self.vars,
            &self.prefills,
            &self.components,
            &self.relations,
            &self.uncomponents,
            &self.opt_components,
        );
        generate_fsm_context(
            &mut result,
            &self.vars,
            &self.prefills,
            &self.components,
            &self.relations,
        );
        generate_invar_archetype_fill(&mut result, &infos, &self.prefills);

        generate_resumable_query_closure(
            &mut result,
            &self.vars,
            &self.prefills,
            &infos,
            &self.relations,
            &self.unequals,
            &self.accessors,
        );

        result.push_str("\n}");
        return result;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct VarInfo {
    /// Index of this variable
    index: isize,
    /// relation type + other var index => index for relation component
    related_with: HashMap<(String, isize), usize>,
    /// indexes in component array for this variables non-optional components
    component_range: Range<usize>,
    /// map from type to component index for accessors
    components: HashMap<String, usize>,
    /// type, index part of context variable name
    opt_components: Vec<(String, usize)>,
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
            .field("opt_components", &self.opt_components)
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

// TODO move to invar node?
pub(crate) fn generate_invar_archetype_fill(
    prepend: &mut String,
    infos: &[VarInfo],
    prefills: &HashMap<isize, String>,
) {
    let mut append = String::new();
    for info in infos.iter().filter(|it| prefills.contains_key(&it.index)) {
        let var_index = info.index;
        let Range { start, end } = &info.component_range;
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
        insert_optional_comps(prepend, &mut append, &info.opt_components);
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

// TODO put building the varinfo into a separate function
pub(crate) fn generate_archetype_sets(
    result: &mut String,
    vars: &[isize],
    prefills: &HashMap<isize, String>,
    components: &[Component],
    relations: &[Relation],
    uncomponents: &[Component],
    opt_components: &[(String, isize, usize)],
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
            opt_components: Vec::new(),
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

        // optional components are not written into archetype set
        // but they are put into the var info
        for (ty, _, index) in opt_components.iter().filter(|(_, id, _)| id == var) {
            info.opt_components.push((ty.clone(), *index));
        }

        infos.push(info);
    }

    if uncomponents.is_empty() {
        result.push_str("let archetype_id_sets = [\n");
        for var in vars {
            if prefills.contains_key(var) {
                // don't need this for prefills
                result.push_str(&format!("    Vec::<ArchetypeId>::new(),\n"));
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
                result.push_str(&format!("    Vec::<ArchetypeId>::new(),\n"));
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
    _prefills: &HashMap<isize, String>, // TODO remove
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
    let (first, invar_unequals, invar_rel_constraints, join_order) =
        compute_join_order(relations, infos, prefills, unequals);

    // TODO save on constants by directly applying the var
    append.push_str(
        "
::std::iter::from_fn(move || { loop { match current_step {",
    );

    let mut step_count;

    if prefills.is_empty() {
        // select first archetype
        let first_info = &infos[first as usize];
        step_count = ArchetypeStart {
            var: first,
            components: first_info.component_range.clone(),
            opt_components: first_info.opt_components.clone(),
        }
        .generate(0, prepend, &mut append);
    } else {
        step_count = InvarStart {
            unequalities: invar_unequals,
            rel_constraints: invar_rel_constraints,
        }
        .generate(0, prepend, &mut append);
    }
    // follow relations/constraints
    for new_join in join_order {
        let NewJoin {
            comp_id: relation_comp,
            old,
            new,
            unequal_constraints,
            rel_constraints,
        } = new_join;
        let info = &infos[new as usize];
        step_count = RelationJoin {
            relation_comp,
            old,
            new,
            new_components: info.component_range.clone(),
            unequal_constraints,
            rel_constraints,
            opt_components: info.opt_components.clone(),
        }
        .generate(step_count, prepend, &mut append);
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
                write!(
                    &mut append,
                    "
            EntityViewDeferred::from_id_unchecked(world,
                                a_refs[{var}].entities[a_rows[{var}].0 as usize]),"
                )
                .unwrap();
            }
            Accessor::OptComponent(ty, var, opt_id) => {
                write!(
                    &mut append,
                    "
            (opt_col_{opt_id}.map(|col| {{
                (&*(col.get(a_rows[{var}].0) as *const RefCell<{ty}>)).borrow()
            }})),"
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
struct NewJoin {
    comp_id: usize,
    old: isize,
    new: isize,
    unequal_constraints: Vec<(isize, isize)>,
    rel_constraints: Vec<(usize, isize, isize)>,
}

// TODO turn into struct
#[derive(Debug)]
enum JoinKind {
    /// component id, old var, new var, unequals to check, relations to check
    NewJoin(NewJoin),
}

fn compute_join_order(
    relations: &[Relation],
    infos: &[VarInfo],
    prefills: &HashMap<isize, String>,
    unequals: &[(isize, isize)],
) -> (
    isize,
    Vec<(isize, isize)>,
    Vec<(usize, isize, isize)>,
    Vec<NewJoin>,
) {
    let mut result = Vec::new();
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
    let newly_available_constraints =
        |available: &mut Vec<isize>, work_left: &mut Vec<Relation>| {
            let mut result = Vec::new();
            while let Some(index) = work_left
                .iter()
                .position(|(_, a, b)| available.contains(a) && available.contains(b))
            {
                let (comp_name, a, b) = work_left[index].clone();
                let info = &infos[a as usize];
                assert_eq!(a, info.index);
                let comp_index = info.related_with[&(comp_name, b)];
                result.push((comp_index, a, b));
                work_left.swap_remove(index);
            }
            result
        };

    let invar_unequals = newly_available_unequals(&mut available);
    let invar_constraints = newly_available_constraints(&mut available, &mut work_left);

    // compute join
    while !work_left.is_empty() {
        // find next viable for joining and remove it from working list
        // always handle constraints first, because it may let us skip work
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
            let old_var = if reversed { join.2 } else { join.1 };
            let new_var = if reversed { join.1 } else { join.2 };
            let info = &infos[old_var as usize];
            assert_eq!(old_var, info.index);
            let comp_id = info.related_with[&(join.0, new_var)];
            available.push(new_var);
            let unequal_constraints = newly_available_unequals(&mut available);
            let relation_constraints = newly_available_constraints(&mut available, &mut work_left);
            result.push(NewJoin {
                comp_id,
                old: old_var,
                new: new_var,
                unequal_constraints,
                rel_constraints: relation_constraints,
            });
        } else {
            panic!("Cross joins are not supported.")
        }
    }
    assert!(unequals.is_empty());

    return (available[0], invar_unequals, invar_constraints, result);
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
        let infos = generate_archetype_sets(
            &mut result,
            &vars,
            &prefills,
            &components,
            &relations,
            &uncomponents,
            &[],
        );
        insta::assert_snapshot!(result, @r#"
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
                opt_components: [],
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
                opt_components: [],
            },
        ]
        "#);

        let join_order = compute_join_order(&relations, &infos, &prefills, &[]);
        insta::assert_debug_snapshot!(join_order, @r#"
        (
            0,
            [],
            [],
            [
                NewJoin {
                    comp_id: 2,
                    old: 0,
                    new: 1,
                    unequal_constraints: [],
                    rel_constraints: [],
                },
            ],
        )
        "#);

        let unequals = vec![(0, 1)];
        let join_order = compute_join_order(&relations, &infos, &prefills, &unequals);
        insta::assert_debug_snapshot!(join_order, @r#"
        (
            0,
            [],
            [],
            [
                NewJoin {
                    comp_id: 2,
                    old: 0,
                    new: 1,
                    unequal_constraints: [
                        (
                            0,
                            1,
                        ),
                    ],
                    rel_constraints: [],
                },
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
        let info = generate_archetype_sets(
            &mut result,
            &vars,
            &prefills,
            &components,
            &relations,
            &uncomponents,
            &[("MyOpt".to_string(), 0, 0)],
        );
        insta::assert_snapshot!(result, @r#"
        let components_0 = [
            world.get_component_id::<Pos>(),
            world.get_component_id::<Speed>(),
        ];

        let archetype_id_sets = [
            bk.matching_archetypes(&components_0, &[]),
        ];
        "#);
        insta::assert_debug_snapshot!(info, @r#"
        [
            VarInfo {
                index: 0,
                related_with: {},
                component_range: 0..2,
                components: {
                    "Pos": 0,
                    "Speed": 1,
                },
                opt_components: [
                    (
                        "MyOpt",
                        0,
                    ),
                ],
            },
        ]
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
            &[],
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
        let opt_components = vec![("Reputation".into(), 1, 0)];

        let mut result = String::new();
        generate_invar_captures(&mut result, &prefills);
        let infos = generate_archetype_sets(
            &mut result,
            &vars,
            &prefills,
            &components,
            &relations,
            &uncomponents,
            &opt_components,
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
            &[],
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

    #[test]
    fn test_optional_component() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let uncomponents = vec![];
        let relations = vec![("Attack".into(), 1, 0)];
        let accessors = vec![
            // TODO opt component access
            Accessor::Component("Unit".to_string(), 0),
            Accessor::Component("Unit".to_string(), 1),
            Accessor::ComponentMut("Health".to_string(), 0),
        ];
        let vars = vec![0, 1];
        let mut result = String::new();
        let prefills = HashMap::new();
        let opt_components = vec![("Reputation".into(), 0, 0)];
        let infos = generate_archetype_sets(
            &mut result,
            &vars,
            &prefills,
            &components,
            &relations,
            &uncomponents,
            &opt_components,
        );
        insta::assert_debug_snapshot!(&infos[0], @r#"
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
            opt_components: [
                (
                    "Reputation",
                    0,
                ),
            ],
        }
        "#);
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
