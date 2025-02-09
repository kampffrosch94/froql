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
pub enum JoinKind {
    InitInvars(InitInvars),
    InitVar(isize),
    InnerJoin(NewJoin),
}

#[derive(Debug)]
pub struct InitInvars {
    pub invar_unequals: Vec<(isize, isize)>,
    pub invar_rel_constraints: Vec<RelationConstraint>,
    pub invar_unrel_constraints: Vec<UnrelationConstraint>,
}

#[derive(Debug)]
pub struct NewJoin {
    pub new: isize,
    pub unequal_constraints: Vec<(isize, isize)>,
    pub rel_constraints: Vec<RelationConstraint>,
    pub unrel_constraints: Vec<UnrelationConstraint>,
}

struct JoinOrderComputer<'a> {
    infos: &'a mut [VarInfo],
    prefills: &'a HashMap<isize, String>,
    work_left: Vec<Relation>,
    unequals: Vec<(isize, isize)>,
    unrelations_left: Vec<Unrelation>,
    available: Vec<isize>,
    result: Vec<JoinKind>,
    init_rank: u32,
    relation_helper_nr: usize,
}

impl<'a> JoinOrderComputer<'a> {
    fn new(
        relations: &'a [Relation],
        infos: &'a mut [VarInfo],
        prefills: &'a HashMap<isize, String>,
        unequals: &'a [(isize, isize)],
        unrelations: &'a [Unrelation],
    ) -> Self {
        let work_left: Vec<Relation> = relations
            .iter()
            .cloned()
            // anyvars only matter as components for constraining archetype sets
            .filter(|(_, from, to)| *from != ANYVAR && *to != ANYVAR)
            .collect();
        let unrelations_left: Vec<Unrelation> = unrelations
            .iter()
            .cloned()
            // anyvars only matter as components for constraining archetype sets
            .filter(|(_, from, to, _)| *from != ANYVAR && *to != ANYVAR)
            .collect();
        Self {
            work_left,
            unrelations_left,
            infos,
            prefills,
            unequals: Vec::from(unequals),
            available: Vec::new(),
            result: Vec::new(),
            init_rank: 0,
            relation_helper_nr: 0,
        }
    }

    fn newly_available_unequals(&mut self) -> Vec<(isize, isize)> {
        let mut result = Vec::new();
        while let Some(index) = self
            .unequals
            .iter()
            .position(|(a, b)| self.available.contains(a) && self.available.contains(b))
        {
            let (a, b) = self.unequals[index];
            result.push((a, b));
            self.unequals.swap_remove(index);
        }
        result
    }

    fn compute_join_order(mut self) -> Vec<JoinKind> {
        // figure out what to start with
        if self.prefills.is_empty() {
            // I think its a decent metric to use the most constrained variable first
            let first = self
                .infos
                .iter_mut()
                .max_by_key(|it| it.component_range.len())
                .unwrap();
            first.init_rank = Some(init_rank);
            init_rank += 1;
            self.available.push(first.index);
            self.result.push(JoinKind::InitVar(first.index));
        } else {
            // if we have prefills we just start with those
            for (var, _) in self.prefills {
                self.available.push(*var);
            }
            self.available.sort();
            for var in &self.available {
                self.infos[*var as usize].init_rank = Some(init_rank);
                init_rank += 1;
            }

            let invar_unequals = self.newly_available_unequals();
            let invar_rel_constraints = newly_available_constraints(
                &self.available,
                &mut self.work_left,
                self.infos,
                &mut relation_helper_nr,
            );
            let invar_unrel_constraints = newly_available_unrelations(
                &self.available,
                &mut self.unrelations_left,
                self.infos,
            );
            self.result.push(JoinKind::InitInvars(InitInvars {
                invar_unequals,
                invar_rel_constraints,
                invar_unrel_constraints,
            }));
        }

        // compute join
        self.compute_inner_joins();
        assert!(self.unequals.is_empty());
        assert!(
            self.unrelations_left.is_empty(),
            "Not all unrelations were inserted."
        );

        return self.result;
    }

    fn compute_inner_joins(&mut self) {
        while !self.work_left.is_empty() {
            // find next viable for joining and remove it from working list
            // always handle constraints first, because it may let us skip work
            // when we are executing the query at runtime
            let next_join = {
                let pos = self.work_left.iter().position(|rel| {
                    self.available
                        .iter()
                        .any(|avail| *avail == rel.1 || *avail == rel.2)
                });
                pos.map(|pos| self.work_left.remove(pos))
            };
            if let Some(join) = next_join {
                let reversed = self.available.iter().any(|avail| *avail == join.2);
                let old_var = if reversed { join.2 } else { join.1 };
                let new_var = if reversed { join.1 } else { join.2 };
                let old_info = &mut self.infos[old_var as usize];
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
                let new_info = &mut self.infos[new_var as usize];
                new_info.join_helper_index = Some(relation_helper_nr);
                new_info.init_rank = Some(init_rank);
                init_rank += 1;
                relation_helper_nr += 1;

                self.available.push(new_var);

                let unequal_constraints = self.newly_available_unequals();
                let mut rel_constraints = newly_available_constraints(
                    &self.available,
                    &mut self.work_left,
                    self.infos,
                    &mut relation_helper_nr,
                );
                let mut unrel_constraints = newly_available_unrelations(
                    &self.available,
                    &mut self.unrelations_left,
                    self.infos,
                );
                for rc in &mut rel_constraints {
                    rc.checked_invar = None; // there must be a better design than this, lol
                }
                for urc in &mut unrel_constraints {
                    urc.checked_invar = None; // there must be a better design than this, lol
                }
                self.result.push(JoinKind::InnerJoin(NewJoin {
                    new: new_var,
                    unequal_constraints,
                    rel_constraints,
                    unrel_constraints,
                }));
            } else {
                panic!("Cross joins are not supported. Use nested queries instead.")
            }
        }
    }
}

pub fn compute_join_order(
    relations: &[Relation],
    infos: &mut [VarInfo],
    prefills: &HashMap<isize, String>,
    unequals: &[(isize, isize)],
    unrelations: &[Unrelation],
) -> Vec<JoinKind> {
    JoinOrderComputer::new(relations, infos, prefills, unequals, unrelations).compute_join_order()
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
