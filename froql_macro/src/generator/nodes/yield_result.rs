use crate::{Accessor, generator::VarInfo};

use super::GeneratorNode;
use std::fmt::Write;

#[derive(Debug)]
pub struct YieldResult<'a> {
    pub accessors: &'a [Accessor],
    pub infos: &'a [VarInfo],
}

impl GeneratorNode for YieldResult<'_> {
    fn generate(&self, step: usize, _prepend: &mut String, append: &mut String) -> usize {
        write!(
            append,
            "
// yield row
{step} => {{
    current_step -= 1;
    return Some(unsafe {{
        ("
        )
        .unwrap();
        for accessor in self.accessors {
            match accessor {
                Accessor::Component(ty, var) => {
                    let col = &self.infos[*var as usize].components[ty];
                    write!(
                        append,
                        "
            ::froql::query_helper::coerce_cast::<{ty}>(
                world,
                a_refs[{var}].columns[col_indexes[{col}]].get(a_rows[{var}].0)
            ).borrow(),"
                    )
                    .unwrap();
                }
                Accessor::ComponentMut(ty, var) => {
                    let col = self.infos[*var as usize].components[ty];
                    write!(
                        append,
                        "
            ::froql::query_helper::coerce_cast::<{ty}>(
                world,
                a_refs[{var}].columns[col_indexes[{col}]].get(a_rows[{var}].0)
            ).borrow_mut(),"
                    )
                    .unwrap();
                }
                Accessor::OutVar(var) => {
                    write!(
                        append,
                        "
            ::froql::entity_view_deferred::EntityViewDeferred::from_id_unchecked(world,
                                a_refs[{var}].entities[a_rows[{var}].0 as usize]),"
                    )
                    .unwrap();
                }
                Accessor::OptComponent(ty, var, opt_id) => {
                    write!(
                        append,
                        "
            (opt_col_{opt_id}.map(|col| {{
                ::froql::query_helper::coerce_cast::<{ty}>(
                    world,
                    col.get(a_rows[{var}].0)
                ).borrow()
            }})),"
                    )
                    .unwrap();
                }
                Accessor::OptMutComponent(ty, var, opt_id) => {
                    write!(
                        append,
                        "
            (opt_col_{opt_id}.map(|col| {{
                ::froql::query_helper::coerce_cast::<{ty}>(
                    world,
                    col.get(a_rows[{var}].0)
                ).borrow_mut()
            }})),"
                    )
                    .unwrap();
                }
                Accessor::Singleton(ty) => {
                    write!(
                        append,
                        "
            world.singleton::<{ty}>(),"
                    )
                    .unwrap();
                }
                Accessor::SingletonMut(ty) => {
                    write!(
                        append,
                        "
            world.singleton_mut::<{ty}>(),"
                    )
                    .unwrap();
                }
            }
        }
        write!(
            append,
            "
        )
    }});
}}
"
        )
        .unwrap();
        usize::MAX // should be terminal
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn relation_join_unequality() {
        let generator = YieldResult {
            accessors: &[Accessor::Component("Health".to_string(), 0)],
            infos: &[VarInfo {
                components: [("Health".to_string(), 0)].into_iter().collect(),
                ..Default::default()
            }],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        generator.generate(3, &mut prepend, &mut append);
        insta::assert_snapshot!(prepend, @"");
        insta::assert_snapshot!(append, @r"
        // yield row
        3 => {
            current_step -= 1;
            return Some(unsafe {
                (
                    ::froql::query_helper::coerce_cast::<Health>(
                        world,
                        a_refs[0].columns[col_indexes[0]].get(a_rows[0].0)
                    ).borrow(),
                )
            });
        }
        ");
    }
}
