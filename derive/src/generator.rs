#![allow(dead_code)] // TODO remove once finished
use std::{collections::HashMap, ops::Range};

use crate::{Component, Relation};

#[derive(Debug)]
struct VarInfo {
    /// other var index, index for relation component
    related_with: HashMap<isize, usize>,
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
            info.related_with.insert(*other, index);
            index += 1;
            info.component_range.end += 1;
        }
        // relation to
        for (ty, other, _) in relations.iter().filter(|(_, _, id)| id == var) {
            result.push_str("\n    ");
            result.push_str(&format!(
                "bk.get_component_id_unchecked(TypeId::of::<Relation<{ty}>>()).flip_target(),"
            ));
            info.related_with.insert(*other, index);
            index += 1;
            info.component_range.end += 1;
        }
        result.push_str("\n];\n\n");
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

fn generate_resumeable_query_closure(
    result: &mut String,
    vars: &[isize],
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

pub fn compute_join_order(relations: &[Relation]) -> Vec<Relation> {
    let mut joined: Vec<Relation> = Vec::new();
    let mut avail: Vec<Relation> = Vec::from(relations);
    joined.push(avail.pop().unwrap());
    for _ in 0..avail.len() {
        let new = avail.iter().position(|(_, c, d)| {
            joined
                .iter()
                .any(|(_, a, b)| b == c || a == c || a == d || b == d)
        });
        if let Some(pos) = new {
            // surely there is a better way to go about this
            // prototype code atm
            let rel = avail.remove(pos);
            joined.push(rel);
        } else {
            panic!("Cross joins are not supported. Use nested queries instead.");
        }
    }
    return joined;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generate_archteype_id_sets_relation() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let vars = vec![0, 1];
        let mut result = String::new();
        insta::assert_snapshot!({
            generate_archetype_sets(&mut result, &vars, &components, &relations);
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
    }

    #[test]
    fn test_generate_archteype_id_sets_trivial() {
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
    fn test_generate_resumeable_query_closure() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let vars = vec![0, 1];
        let mut result = String::new();
        insta::assert_snapshot!({
            generate_resumeable_query_closure(&mut result, &vars, &components, &relations);
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
        let relations = vec![("Attack".into(), 1, 0), ("Bla".into(), 0, 1)];
        let joined = compute_join_order(&relations);
        let s = joined
            .iter()
            .map(|(ty, from, to)| format!("({ty}, {from}, {to})"))
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(s, @r#"
        (Bla, 0, 1)
        (Attack, 1, 0)
        "#);
    }
}
