use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::fmt::Write;
use std::{collections::HashMap, ops::Range};

use join_order::compute_join_order;
use join_order::InitInvars;
use join_order::InitVar;
use join_order::JoinKind;
use join_order::NewJoin;

use crate::generator_nodes::archetype_start::ArchetypeStart;
use crate::generator_nodes::invar_start::InvarInfo;
use crate::generator_nodes::invar_start::InvarStart;
use crate::generator_nodes::relation_helper::RelationHelperInfo;
use crate::generator_nodes::relation_helper::UnrelationHelperInfo;
use crate::generator_nodes::relation_join::RelationJoin;
use crate::generator_nodes::GeneratorNode;
use crate::Unrelation;
use crate::ANYVAR;
use crate::{Accessor, Component, Relation};
mod join_order;
pub use join_order::Checks;

#[derive(Default, Debug)]
pub struct Generator {
    pub vars: Vec<isize>,
    pub prefills: HashMap<isize, String>,
    pub components: Vec<Component>,
    pub relations: Vec<Relation>,
    pub uncomponents: Vec<Component>,
    /// Type, variable, opt component number
    pub opt_components: Vec<(String, isize, usize)>,
    pub unequals: Vec<(isize, isize)>,
    pub accessors: Vec<Accessor>,
    pub unrelations: Vec<Unrelation>,
}

impl Generator {
    pub fn generate(&self, world: &str) -> String {
        let mut result = String::new();

        result.push_str("{\n");

        generate_invar_captures(&mut result, &self.prefills);

        write!(
            &mut result,
            "
let world: &World = &{world};
let bk = &world.bookkeeping;
"
        )
        .unwrap();

        let mut infos = generate_archetype_sets(
            &mut result,
            &self.vars,
            &self.prefills,
            &self.components,
            &self.relations,
            &self.uncomponents,
            &self.opt_components,
            &self.unrelations,
        );
        generate_fsm_context(&mut result, &self.vars, &self.components, &self.relations);

        generate_resumable_query_closure(
            &mut result,
            &self.vars,
            &self.prefills,
            &mut infos,
            &self.relations,
            &self.unequals,
            &self.accessors,
            &self.unrelations,
        );

        result.push_str("\n}");
        return result;
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct VarInfo {
    /// Index of this variable
    index: isize,
    /// variables are intialized in this order
    /// useful for finding out which one is initialized earlier
    /// None when rank is not decided yet
    init_rank: Option<u32>,
    /// relation type + other var index => index for relation component
    related_with: HashMap<(String, isize), usize>,
    /// indexes in component array for this variables non-optional components
    component_range: Range<usize>,
    /// map from type to component index for accessors
    components: HashMap<String, usize>,
    /// type, index part of context variable name
    opt_components: Vec<(String, usize)>,
    /// only built up when joins are computed
    /// When a join is added the already existing variable (`old`) gets a relationship helper added.
    /// This is then used for code gen in steps leading up to the join.
    relation_helpers: Vec<RelationHelperInfo>,
    /// Unrelationhelpers are optional (!) RelationHelpers that are negated in checks.
    unrelation_helpers: Vec<UnrelationHelperInfo>,
    /// if this var is set by a relation join, then this is the index of the RelationHelper
    /// for that join
    join_helper_index: Option<usize>,
}

impl Debug for VarInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // fix ordering, for snapshot testing
        let related_with = self.related_with.iter().collect::<BTreeMap<_, _>>();
        let components = self.components.iter().collect::<BTreeMap<_, _>>();
        f.debug_struct("VarInfo")
            .field("index", &self.index)
            .field("init_rank", &self.init_rank)
            .field("related_with", &related_with)
            .field("component_range", &self.component_range)
            .field("components", &components)
            .field("opt_components", &self.opt_components)
            .field("relation_helpers", &self.relation_helpers)
            .field("unrelation_helpers", &self.unrelation_helpers)
            .field("join_helper_index", &self.join_helper_index)
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
            "let invar_{index}: ::froql::entity_store::Entity = (&{name}).into();
"
        )
        .unwrap();
    }
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
    unrelations: &[Unrelation], // only care about unrelations with anyvars here
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
            relation_helpers: Vec::new(),
            join_helper_index: None,
            init_rank: None,
            unrelation_helpers: Vec::new(),
        };
        write!(result, "let components_{var} = [").unwrap();
        // component
        let mut dedup = HashSet::new();
        for (ty, _) in components.iter().filter(|(_, id)| id == var) {
            if dedup.contains(&ty) {
                continue;
            }
            dedup.insert(ty);

            write!(result, "\n    world.get_component_id::<{ty}>(),").unwrap();
            info.components.insert(ty.clone(), index);
            index += 1;
            info.component_range.end += 1;
        }

        // relation from
        dedup.clear();
        for (ty, _, other) in relations.iter().filter(|(_, id, _)| id == var) {
            if dedup.contains(&ty) {
                continue;
            }
            dedup.insert(ty);

            write!(
                result,
                "\n    bk.get_component_id_unchecked(::std::any::TypeId::of::<::froql::relation::Relation<{ty}>>()),"
            )
            .unwrap();
            info.related_with.insert((ty.clone(), *other), index);
            index += 1;
            info.component_range.end += 1;
        }
        // relation to
        dedup.clear();
        for (ty, other, _) in relations.iter().filter(|(_, _, id)| id == var) {
            if dedup.contains(&ty) {
                continue;
            }
            dedup.insert(ty);

            result.push_str("\n    ");
            write!(
                result,
                "bk.get_component_id_unchecked(::std::any::TypeId::of::<::froql::relation::Relation<{ty}>>()).flip_target(),"
            )
            .unwrap();
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

    if uncomponents.is_empty() && unrelations.is_empty() {
        result.push_str("let archetype_id_sets = [\n");
        for var in vars {
            if prefills.contains_key(var) {
                // don't need this for prefills
                result.push_str("    Vec::<::froql::archetype::ArchetypeId>::new(),\n");
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

            write!(result, "let uncomponents_{var} = [").unwrap();

            // component
            for (ty, _) in uncomponents.iter().filter(|(_, id)| id == var) {
                write!(result, "\n    world.get_component_id::<{ty}>(),").unwrap();
            }

            // unrelations from var to anyvar
            for (ty, _, _, _) in unrelations
                .iter()
                .filter(|(_, id, any, _)| *any == ANYVAR && id == var)
            {
                write!(
                    result,
                    "\n    bk.get_component_id_unchecked(::std::any::TypeId::of::<::froql::relation::Relation<{ty}>>()),"
                )
                .unwrap();
            }

            // unrelations from anyvar to var
            for (ty, _, _, _) in unrelations
                .iter()
                .filter(|(_, any, id, _)| *any == ANYVAR && id == var)
            {
                result.push_str("\n    ");
                write!(
                    result,
                    "bk.get_component_id_unchecked(::std::any::TypeId::of::<::froql::relation::Relation<{ty}>>()).flip_target(),"
                )
                .unwrap();
            }

            result.push_str("\n];\n\n");
        }
        result.push_str("let archetype_id_sets = [\n");
        for var in vars {
            if prefills.contains_key(var) {
                // don't need this for prefills
                write!(
                    result,
                    "    Vec::<::froql::archetype::ArchetypeId>::new(),\n"
                )
                .unwrap();
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
    components: &[Component],
    relations: &[Relation],
) {
    let var_count = vars.len();
    let col_count = components.len() + relations.len() * 2;
    write!(
        result,
        "
// result set
const VAR_COUNT: usize = {var_count};
let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];
let mut a_rows = [::froql::archetype::ArchetypeRow(u32::MAX); VAR_COUNT];

// general context for statemachine
let mut current_step = 0;
let mut a_max_rows = [0; VAR_COUNT];
let mut a_next_indexes = [usize::MAX; VAR_COUNT];
let mut col_indexes = [usize::MAX; {col_count}];
"
    )
    .unwrap();
}

pub fn generate_resumable_query_closure(
    result: &mut String,
    vars: &[isize],
    prefills: &HashMap<isize, String>,
    infos: &mut [VarInfo],
    relations: &[Relation],
    unequals: &[(isize, isize)],
    accessors: &[Accessor],
    unrelations: &[Unrelation],
) {
    assert_eq!(infos.len(), vars.len());
    let prepend = result;
    let mut append = String::new();
    let join_order = compute_join_order(relations, infos, prefills, unequals, unrelations);

    append.push_str(
        "
::std::iter::from_fn(move || { loop { match current_step {",
    );

    let mut step_count = 0;

    for join in join_order {
        match join {
            JoinKind::InitInvars(init_invars) => {
                let InitInvars {
                    invar_unequals,
                    invar_rel_constraints,
                    invar_unrel_constraints,
                } = init_invars;
                step_count = InvarStart {
                    unequalities: invar_unequals,
                    rel_constraints: invar_rel_constraints,
                    unrel_constraints: invar_unrel_constraints,
                    invars: infos
                        .iter()
                        .filter(|it| prefills.contains_key(&it.index))
                        .map(|info| InvarInfo {
                            var_index: info.index,
                            component_range: info.component_range.clone(),
                            opt_components: info.opt_components.clone(),
                            relation_helpers: info.relation_helpers.clone(),
                            unrelation_helpers: info.unrelation_helpers.clone(),
                        })
                        .collect(),
                }
                .generate(step_count, prepend, &mut append);
            }
            JoinKind::InitVar(InitVar { var, checks }) => {
                // select first archetype
                let first_info = &infos[var as usize];
                step_count = ArchetypeStart {
                    var,
                    components: first_info.component_range.clone(),
                    opt_components: first_info.opt_components.clone(),
                    relation_helpers: first_info.relation_helpers.clone(),
                    unrelation_helpers: first_info.unrelation_helpers.clone(),
                    checks,
                }
                .generate(step_count, prepend, &mut append);
            }
            JoinKind::InnerJoin(new_join) => {
                let NewJoin {
                    new,
                    unequal_constraints,
                    rel_constraints,
                    unrel_constraints,
                } = new_join;
                let new_info = &infos[new as usize];
                step_count = RelationJoin {
                    new,
                    new_components: new_info.component_range.clone(),
                    unequal_constraints,
                    rel_constraints,
                    unrel_constraints,
                    opt_components: new_info.opt_components.clone(),
                    new_relation_helpers: new_info.relation_helpers.clone(),
                    new_helper_nr: new_info
                        .join_helper_index
                        .expect("Internal: RelationHelper needs to exist for Join"),
                    new_unrelation_helpers: new_info.unrelation_helpers.clone(),
                }
                .generate(step_count, prepend, &mut append);
            }
        };
    }

    // TODO put yield into generator node
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
                as *const ::std::cell::RefCell<{ty}>))
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
                as *const ::std::cell::RefCell<{ty}>))
                .borrow_mut(),"
                )
                .unwrap();
            }
            Accessor::OutVar(var) => {
                write!(
                    &mut append,
                    "
            ::froql::entity_view_deferred::EntityViewDeferred::from_id_unchecked(world,
                                a_refs[{var}].entities[a_rows[{var}].0 as usize]),"
                )
                .unwrap();
            }
            Accessor::OptComponent(ty, var, opt_id) => {
                write!(
                    &mut append,
                    "
            (opt_col_{opt_id}.map(|col| {{
                (&*(col.get(a_rows[{var}].0) as *const ::std::cell::RefCell<{ty}>)).borrow()
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

#[cfg(test)]
mod test {
    use join_order::compute_join_order;

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
        let mut infos = generate_archetype_sets(
            &mut result,
            &vars,
            &prefills,
            &components,
            &relations,
            &uncomponents,
            &[],
            &[],
        );
        insta::assert_snapshot!(result, @r#"
        let components_0 = [
            world.get_component_id::<Unit>(),
            world.get_component_id::<Health>(),
            bk.get_component_id_unchecked(::std::any::TypeId::of::<::froql::relation::Relation<Attack>>()).flip_target(),
        ];

        let components_1 = [
            world.get_component_id::<Unit>(),
            bk.get_component_id_unchecked(::std::any::TypeId::of::<::froql::relation::Relation<Attack>>()),
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
                init_rank: None,
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
                relation_helpers: [],
                unrelation_helpers: [],
                join_helper_index: None,
            },
            VarInfo {
                index: 1,
                init_rank: None,
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
                relation_helpers: [],
                unrelation_helpers: [],
                join_helper_index: None,
            },
        ]
        "#);

        let join_order = compute_join_order(&relations, &mut infos, &prefills, &[], &[]);
        insta::assert_debug_snapshot!(join_order, @r#"
        [
            InitVar(
                InitVar {
                    var: 0,
                    checks: Checks {
                        unequals: [],
                        rel_constraints: [],
                        unrel_constraints: [],
                    },
                },
            ),
            InnerJoin(
                NewJoin {
                    new: 1,
                    unequal_constraints: [],
                    rel_constraints: [],
                    unrel_constraints: [],
                },
            ),
        ]
        "#);

        let unequals = vec![(0, 1)];
        let join_order = compute_join_order(&relations, &mut infos, &prefills, &unequals, &[]);
        insta::assert_debug_snapshot!(join_order, @r#"
        [
            InitVar(
                InitVar {
                    var: 0,
                    checks: Checks {
                        unequals: [],
                        rel_constraints: [],
                        unrel_constraints: [],
                    },
                },
            ),
            InnerJoin(
                NewJoin {
                    new: 1,
                    unequal_constraints: [
                        (
                            0,
                            1,
                        ),
                    ],
                    rel_constraints: [],
                    unrel_constraints: [],
                },
            ),
        ]
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
        let invar_1: ::froql::entity_store::Entity = player.into();
        let invar_2: ::froql::entity_store::Entity = somebody.into();
        "#);

        let mut result = String::new();
        let mut prefills = HashMap::new();
        prefills.insert(1, "player".to_string());
        insta::assert_snapshot!({
            generate_invar_captures(&mut result, &prefills);
            result
        }, @"let invar_1: ::froql::entity_store::Entity = player.into();");

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
        let mut a_rows = [::froql::archetype::ArchetypeRow(u32::MAX); VAR_COUNT];

        // general context for statemachine
        let mut current_step = 0;
        let mut a_max_rows = [0; VAR_COUNT];
        let mut a_next_indexes = [usize::MAX; VAR_COUNT];
        let mut col_indexes = [usize::MAX; 5];
        "#);
    }

    #[test]
    fn test_relation_outvar() {
        let vars = vec![0, 1];
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let accessors = vec![
            Accessor::OutVar(0),
            Accessor::Component("Unit".to_string(), 0),
            Accessor::Component("Unit".to_string(), 1),
            Accessor::ComponentMut("Health".to_string(), 0),
        ];
        let result = Generator {
            vars,
            components,
            relations,
            accessors,
            ..Default::default()
        }
        .generate("world");

        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_invar() {
        let vars = vec![0, 1];
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let accessors = vec![
            Accessor::Component("Unit".to_string(), 0),
            Accessor::Component("Unit".to_string(), 1),
            Accessor::ComponentMut("Health".to_string(), 0),
        ];
        let prefills = vec![(1, "player".to_string())].into_iter().collect();
        let opt_components = vec![("Reputation".into(), 1, 0)];

        let result = Generator {
            vars,
            components,
            relations,
            accessors,
            prefills,
            opt_components,
            ..Default::default()
        }
        .generate("world");

        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_unequals() {
        let vars = vec![0, 1];
        let unequals = vec![(0, 1)];
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let accessors = vec![
            Accessor::OutVar(0),
            Accessor::Component("Unit".to_string(), 0),
            Accessor::Component("Unit".to_string(), 1),
            Accessor::ComponentMut("Health".to_string(), 0),
        ];

        let result = Generator {
            vars,
            unequals,
            components,
            relations,
            accessors,
            ..Default::default()
        }
        .generate("world");

        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_optional_component() {
        let vars = vec![0, 1];
        let unequals = vec![(0, 1)];
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let opt_components = vec![("Reputation".into(), 0, 0)];
        let accessors = vec![
            Accessor::OptComponent("Reputation".into(), 0, 0),
            Accessor::Component("Unit".to_string(), 0),
            Accessor::Component("Unit".to_string(), 1),
            Accessor::ComponentMut("Health".to_string(), 0),
        ];

        let result = Generator {
            vars,
            components,
            relations,
            accessors,
            opt_components,
            unequals,
            ..Default::default()
        }
        .generate("world");

        insta::assert_snapshot!(result);
    }
}
