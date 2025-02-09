use std::collections::HashMap;

use crate::{
    generator_nodes::{
        relation_helper::{RelationHelperInfo, UnrelationHelperInfo},
        types::{RelationConstraint, UnrelationConstraint},
    },
    Relation, Unrelation, ANYVAR,
};

use super::VarInfo;

#[derive(Debug)]
pub struct NewJoin {
    pub new: isize,
    pub unequal_constraints: Vec<(isize, isize)>,
    pub rel_constraints: Vec<RelationConstraint>,
    pub unrel_constraints: Vec<UnrelationConstraint>,
}

#[derive(Debug)]
pub struct JoinOrder {
    pub first: isize,
    pub invar_unequals: Vec<(isize, isize)>,
    pub invar_rel_constraints: Vec<RelationConstraint>,
    pub invar_unrel_constraints: Vec<UnrelationConstraint>,
    pub join_order: Vec<NewJoin>,
}

pub fn compute_join_order(
    relations: &[Relation],
    infos: &mut [VarInfo],
    prefills: &HashMap<isize, String>,
    unequals: &[(isize, isize)],
    unrelations: &[Unrelation],
) -> JoinOrder {
    let mut result = Vec::new();
    let mut available: Vec<isize> = Vec::new();
    let mut unequals = Vec::from(unequals);
    let mut work_left: Vec<Relation> = relations
        .iter()
        .cloned()
        // anyvars only matter as components for constraining archetype sets
        .filter(|(_, from, to)| *from != ANYVAR && *to != ANYVAR)
        .collect();
    let mut unrelations_left: Vec<Unrelation> = unrelations
        .iter()
        .cloned()
        // anyvars only matter as components for constraining archetype sets
        .filter(|(_, from, to, _)| *from != ANYVAR && *to != ANYVAR)
        .collect();

    // figure out what to start with
    let mut init_rank = 0;
    if prefills.is_empty() {
        // I think its a decent metric to use the most constrained variable first
        let first = infos
            .iter_mut()
            .max_by_key(|it| it.component_range.len())
            .unwrap();
        first.init_rank = Some(init_rank);
        init_rank += 1;
        available.push(first.index);
    } else {
        // if we have prefills we just start with those
        for (var, _) in prefills {
            available.push(*var);
        }
        available.sort();
        for var in &available {
            infos[*var as usize].init_rank = Some(init_rank);
            init_rank += 1;
        }
    }

    let mut relation_helper_nr = 0;

    // using a closure so I don't have to duplicate this part
    let mut newly_available_unequals = |available: &Vec<isize>| {
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

    let invar_unequals = newly_available_unequals(&available);
    let invar_rel_constraints =
        newly_available_constraints(&available, &mut work_left, infos, &mut relation_helper_nr);
    let invar_unrel_constraints =
        newly_available_unrelations(&available, &mut unrelations_left, infos);

    // compute join
    while !work_left.is_empty() {
        // find next viable for joining and remove it from working list
        // always handle constraints first, because it may let us skip work
        // when we are executing the query at runtime
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
            let old_info = &mut infos[old_var as usize];
            assert_eq!(old_var, old_info.index);
            let column_index = old_info.related_with[&(join.0, new_var)];
            let cid_index = column_index - old_info.component_range.start;

            old_info.relation_helpers.push(RelationHelperInfo {
                column_index,
                old_var,
                new_var,
                nr: relation_helper_nr,
                cid_index,
            });
            let new_info = &mut infos[new_var as usize];
            new_info.join_helper_index = Some(relation_helper_nr);
            new_info.init_rank = Some(init_rank);
            init_rank += 1;
            relation_helper_nr += 1;

            available.push(new_var);

            let unequal_constraints = newly_available_unequals(&mut available);
            let mut rel_constraints = newly_available_constraints(
                &available,
                &mut work_left,
                infos,
                &mut relation_helper_nr,
            );
            let mut unrel_constraints =
                newly_available_unrelations(&available, &mut unrelations_left, infos);
            for rc in &mut rel_constraints {
                rc.checked_invar = None; // there must be a better design than this, lol
            }
            for urc in &mut unrel_constraints {
                urc.checked_invar = None; // there must be a better design than this, lol
            }
            result.push(NewJoin {
                new: new_var,
                unequal_constraints,
                rel_constraints,
                unrel_constraints,
            });
        } else {
            panic!("Cross joins are not supported. Use nested queries instead.")
        }
    }
    assert!(unequals.is_empty());
    assert!(
        unrelations_left.is_empty(),
        "Not all unrelations were inserted {unrelations_left:?} \n{available:?}"
    );

    return JoinOrder {
        first: available[0],
        invar_unequals,
        invar_rel_constraints,
        invar_unrel_constraints,
        join_order: result,
    };
}

fn newly_available_constraints(
    available: &Vec<isize>,
    relations_left: &mut Vec<Relation>,
    infos: &mut [VarInfo],
    relation_helper_nr: &mut usize,
) -> Vec<RelationConstraint> {
    let mut result = Vec::new();
    while let Some(index) = relations_left
        .iter()
        .position(|(_, a, b)| available.contains(a) && available.contains(b))
    {
        let (comp_name, a, b) = relations_left[index].clone();
        let (old, new) =
            if infos[a as usize].init_rank.unwrap() < infos[b as usize].init_rank.unwrap() {
                (a, b)
            } else {
                (b, a)
            };
        let old_info = &mut infos[old as usize];
        assert_eq!(old, old_info.index);
        let column_index = old_info.related_with[&(comp_name, new)];
        result.push(RelationConstraint {
            helper_nr: *relation_helper_nr,
            checked_invar: Some(new),
        });
        relations_left.swap_remove(index);

        let cid_index = column_index - old_info.component_range.start;
        old_info.relation_helpers.push(RelationHelperInfo {
            column_index,
            old_var: old,
            new_var: new,
            nr: *relation_helper_nr,
            cid_index,
        });
        let new_info = &mut infos[new as usize];
        new_info.join_helper_index = Some(*relation_helper_nr);
        *relation_helper_nr += 1;
    }
    return result;
}

fn newly_available_unrelations(
    available: &Vec<isize>,
    unrelations_left: &mut Vec<Unrelation>,
    infos: &mut [VarInfo],
) -> Vec<UnrelationConstraint> {
    let mut result = Vec::new();
    while let Some(index) = unrelations_left
        .iter()
        .position(|(_, a, b, _)| available.contains(a) && available.contains(b))
    {
        let (ty, a, b, number) = unrelations_left[index].clone();
        let (old, new, flip) =
            if infos[a as usize].init_rank.unwrap() < infos[b as usize].init_rank.unwrap() {
                (a, b, false)
            } else {
                (b, a, true)
            };
        let old_info = &mut infos[old as usize];
        assert_eq!(old, old_info.index);
        result.push(UnrelationConstraint {
            helper_nr: number,
            checked_invar: Some(new),
        });
        unrelations_left.swap_remove(index);

        old_info.unrelation_helpers.push(UnrelationHelperInfo {
            ty,
            flip_target: flip,
            old_var: old,
            new_var: new,
            nr: number,
        });
        let new_info = &mut infos[new as usize];
        new_info.join_helper_index = Some(number);
    }
    return result;
}
