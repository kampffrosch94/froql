use crate::{Component, Relation};

fn generate_archetype_sets(
    vars: &[isize],
    component: &[Component],
    relations: &[Relation],
) -> String {
    assert_ne!(0, component.len() + relations.len());
    assert_ne!(0, vars.len());

    let mut result = String::new();

    for var in vars {
        result.push_str(&format!("let components_{var} = ["));
        for (ty, _) in component.iter().filter(|(_, id)| id == var) {
            result.push_str(&format!("\n    world.get_component_id::<{ty}>(),"));
        }
        // relation from
        for (ty, _, _) in relations.iter().filter(|(_, id, _)| id == var) {
            result.push_str(&format!(
                "\n    bk.get_component_id_unchecked(TypeId::of::<Relation<{ty}>>()),"
            ));
        }
        // relation to
        for (ty, _, _) in relations.iter().filter(|(_, _, id)| id == var) {
            result.push_str("\n    ");
            result.push_str(&format!(
                "bk.get_component_id_unchecked(TypeId::of::<Relation<{ty}>>()).flip_target(),"
            ));
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
    result
}


fn generate_fsm_context(
    vars: &[isize],
    component: &[Component],
    relations: &[Relation],
) -> String {
    let var_count = vars.len();
    let col_count = component.len() + relations.len() * 2;
    let mut result = format!("
// result set
const VAR_COUNT: usize = {var_count};
let mut a_refs = [&bk.archetypes[0]; VAR_COUNT];
let mut a_rows = [ArchetypeRow(u32::MAX); VAR_COUNT];

// general context for statemachine
let mut current_step = 0;
let mut a_max_rows = [0; VAR_COUNT];
let mut a_next_indexes = [usize::MAX; VAR_COUNT];
let mut col_indexes = [usize::MAX; {col_count}];

let mut rel_index_2 = 0; <================= TODO
");
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generate_archteype_id_sets_relation() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let vars = vec![0, 1];
        insta::assert_snapshot!(generate_archetype_sets(&vars, &components, &relations));
    }

    #[test]
    fn test_generate_archteype_id_sets_trivial() {
        let components = vec![("Pos".into(), 0), ("Speed".into(), 0)];
        let relations = [];
        let vars = vec![0];
        insta::assert_snapshot!(generate_archetype_sets(&vars, &components, &relations));
    }

    #[test]
    fn test_generate_result_set() {
        let components = vec![("Unit".into(), 0), ("Health".into(), 0), ("Unit".into(), 1)];
        let relations = vec![("Attack".into(), 1, 0)];
        let vars = vec![0, 1];
        insta::assert_snapshot!(generate_fsm_context(&vars,&components, &relations));
    }
}
