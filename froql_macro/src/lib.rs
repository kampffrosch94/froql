extern crate proc_macro;

mod generator;
mod macro_error;
mod parser;

use std::collections::HashMap;

use crate::generator::*;
use macro_error::MacroError;
use parser::RelationVarKind as RVK;
use parser::VarKind as VK;
use parser::{parse_term, Term};
use proc_macro::{TokenStream, TokenTree};
use std::fmt::Write;

const ANYVAR: isize = isize::MAX;

#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
    return match inner(input) {
        Ok(tt) => tt,
        Err(err) => err.to_compile_error(),
    };
}

/// RelationType, from_var, to_var
pub(crate) type Relation = (String, isize, isize);
/// ComponentType, source_var
pub(crate) type Component = (String, isize);

// we need to preserve the order of the query in the result
// this is why we put result entities and components in the same vec via enum
pub(crate) enum Accessor {
    /// ComponentType, index_in_result_array, var
    Component(String, isize),
    /// ComponentType, index_in_result_array, var
    ComponentMut(String, isize),
    /// var index in result
    OutVar(isize),
}

struct VariableStore {
    variables: HashMap<String, isize>,
    var_count: isize,
}

impl VariableStore {
    fn new() -> Self {
        Self {
            variables: Default::default(),
            var_count: -1, // we add 1 before returning a number
        }
    }

    fn var_number(&mut self, var_name: impl Into<String>) -> isize {
        let var = *self.variables.entry(var_name.into()).or_insert_with(|| {
            self.var_count += 1;
            self.var_count
        });
        var
    }
}

#[allow(unused)] // TODO remove once cleaned up
fn inner(input: TokenStream) -> Result<TokenStream, MacroError> {
    //dbg!(&input);

    let mut iter = input.into_iter();

    let world = if let Some(TokenTree::Ident(world)) = iter.next() {
        world.to_string()
    } else {
        panic!("First argument should be a reference to the world.");
    };

    if let Some(TokenTree::Punct(comma)) = iter.next() {
        assert_eq!(',', comma.as_char(), "Expected , after world");
    } else {
        panic!("Expected , after world");
    };

    let mut buffer: Vec<TokenTree> = Vec::with_capacity(10);
    let mut variables = VariableStore::new();

    let mut components: Vec<Component> = Vec::new();
    let mut uncomponents: Vec<Component> = Vec::new();
    let mut accessors: Vec<Accessor> = Vec::new();
    let mut relations: Vec<Relation> = Vec::new();

    let mut unequals = Vec::new();
    let mut unrelations = Vec::new();
    let mut prefills = HashMap::new();

    loop {
        let next = iter.next();

        let is_separator = match next {
            None => true,
            Some(TokenTree::Punct(ref comma)) if comma.as_char() == ',' => true,
            _ => false,
        };
        if is_separator && buffer.len() > 0 {
            match parse_term(&buffer).map(transform_anyvars)? {
                Term::ComponentVar(ty, ref varkind @ VK::Var(ref var_name))
                | Term::ComponentVar(ty, ref varkind @ VK::InVar(ref var_name)) => {
                    let var = variables.var_number(var_name);
                    match varkind {
                        VK::Var(_) => (),
                        VK::InVar(_) => {
                            // TODO handle override with different name => error
                            prefills.insert(var, var_name.clone());
                        }
                    }
                    components.push((ty.clone(), var));
                    accessors.push(Accessor::Component(ty, var));
                }
                Term::MutComponentVar(ty, ref varkind @ VK::Var(ref var_name))
                | Term::MutComponentVar(ty, ref varkind @ VK::InVar(ref var_name)) => {
                    let var = variables.var_number(var_name);
                    match varkind {
                        VK::Var(_) => (),
                        VK::InVar(_) => {
                            prefills.insert(var, var_name.clone());
                        }
                    }
                    components.push((ty.clone(), var));
                    accessors.push(Accessor::ComponentMut(ty, var));
                }
                Term::NoOutComponentVar(ty, ref varkind @ VK::Var(ref var_name))
                | Term::NoOutComponentVar(ty, ref varkind @ VK::InVar(ref var_name)) => {
                    // Optimization: don't need to access this, just check its there
                    let var = variables.var_number(var_name);
                    match varkind {
                        VK::Var(_) => (),
                        VK::InVar(_) => {
                            prefills.insert(var, var_name.clone());
                        }
                    }
                    components.push((ty, var));
                }
                Term::OutVar(var) => {
                    let var = variables.var_number(var);
                    accessors.push(Accessor::OutVar(var));
                }
                Term::ConstraintUnequal(
                    ref ta @ VK::Var(ref var_a),
                    ref tb @ VK::Var(ref var_b),
                )
                | Term::ConstraintUnequal(
                    ref ta @ VK::InVar(ref var_a),
                    ref tb @ VK::Var(ref var_b),
                )
                | Term::ConstraintUnequal(
                    ref ta @ VK::Var(ref var_a),
                    ref tb @ VK::InVar(ref var_b),
                ) => {
                    // maybe we should error if a constraint uses a variable defined nowhere else?
                    let a = variables.var_number(var_a);
                    let b = variables.var_number(var_b);
                    match (ta, tb) {
                        (VK::InVar(_), VK::Var(_)) => {
                            prefills.insert(a, var_a.clone());
                        }
                        (VK::Var(_), VK::InVar(_)) => {
                            prefills.insert(b, var_b.clone());
                        }
                        (VK::Var(_), VK::Var(_)) | (VK::InVar(_), VK::InVar(_)) => (),
                    }
                    unequals.push(format!("({a}, {b})"));
                }
                Term::ConstraintUnequal(VK::InVar(_), VK::InVar(_)) => {
                    panic!("*a != *b can be done outside the query")
                }
                Term::Uncomponent(ty, var) => {
                    let var = variables.var_number(var);
                    uncomponents.push((ty, var));
                }
                Term::Relation(ty, RVK::Var(var_a), RVK::Var(var_b)) => {
                    let a = variables.var_number(var_a);
                    let b = variables.var_number(var_b);
                    relations.push((ty, a, b));
                }
                Term::Relation(ty, RVK::InVar(var_a), RVK::Var(var_b)) => {
                    let a = variables.var_number(&var_a);
                    let b = variables.var_number(var_b);
                    relations.push((ty, a, b));
                    prefills.insert(a, var_a);
                }
                Term::Relation(_, RVK::InVar(_), RVK::InVar(_)) => {
                    unreachable!("The parser should never get us here.");
                }

                Term::Relation(ty, RVK::Var(var_a), RVK::InVar(var_b)) => {
                    let a = variables.var_number(var_a);
                    let b = variables.var_number(&var_b);
                    relations.push((ty, a, b));
                    prefills.insert(b, var_b);
                }
                Term::Relation(ty, RVK::Var(var_a), RVK::AnyVar) => {
                    let a = variables.var_number(var_a);
                    let b = ANYVAR;
                    todo!("Relation Var AnyVar");
                    relations.push((ty, a, b));
                }
                Term::Relation(ty, RVK::InVar(var_a), RVK::AnyVar) => {
                    let a = variables.var_number(&var_a);
                    let b = ANYVAR;
                    prefills.insert(a, var_a);
                    relations.push((ty, a, b));
                    todo!("Relation Invar AnyVar");
                }
                Term::Relation(ty, RVK::AnyVar, RVK::Var(var_b)) => {
                    let a = ANYVAR;
                    let b = variables.var_number(var_b);
                    relations.push((ty, a, b));
                    todo!("Relation AnyVar Var");
                }
                Term::Relation(ty, RVK::AnyVar, RVK::InVar(var_b)) => {
                    let a = ANYVAR;
                    let b = variables.var_number(&var_b);
                    prefills.insert(b, var_b);
                    relations.push((ty, a, b));
                    todo!("Relation AnyVar InVar");
                }
                Term::Relation(_ty, RVK::AnyVar, RVK::AnyVar) => {
                    panic!("Rel(_,_) does not make sense.");
                }
                Term::Unrelation(
                    ty,
                    ref term_a @ RVK::Var(ref var_a),
                    ref term_b @ RVK::Var(ref var_b),
                )
                | Term::Unrelation(
                    ty,
                    ref term_a @ RVK::InVar(ref var_a),
                    ref term_b @ RVK::Var(ref var_b),
                )
                | Term::Unrelation(
                    ty,
                    ref term_a @ RVK::Var(ref var_a),
                    ref term_b @ RVK::InVar(ref var_b),
                ) => {
                    let a = variables.var_number(var_a);
                    let b = variables.var_number(var_b);
                    match (term_a, term_b) {
                        (RVK::InVar(_), RVK::Var(_)) => {
                            prefills.insert(a, var_a.clone());
                        }
                        (RVK::Var(_), RVK::InVar(_)) => {
                            prefills.insert(b, var_b.clone());
                        }
                        _ => (),
                    }
                    unrelations.push(format!("(TypeId::of::<{ty}>(), {a}, {b})"));
                }
                Term::Unrelation(ty, RVK::Var(var_a), RVK::AnyVar) => {
                    let a = variables.var_number(var_a);
                    let b = ANYVAR;
                    unrelations.push(format!("(TypeId::of::<{ty}>(), {a}, {b})"));
                }
                Term::Unrelation(ty, RVK::InVar(var_a), RVK::AnyVar) => {
                    let a = variables.var_number(&var_a);
                    let b = ANYVAR;
                    prefills.insert(a, var_a);
                    unrelations.push(format!("(TypeId::of::<{ty}>(), {a}, {b})"));
                }
                Term::Unrelation(ty, RVK::AnyVar, RVK::Var(var_b)) => {
                    let a = ANYVAR;
                    let b = variables.var_number(var_b);
                    unrelations.push(format!("(TypeId::of::<{ty}>(), {a}, {b})"));
                }
                Term::Unrelation(ty, RVK::AnyVar, RVK::InVar(var_b)) => {
                    let a = ANYVAR;
                    let b = variables.var_number(&var_b);
                    prefills.insert(b, var_b);
                    unrelations.push(format!("(TypeId::of::<{ty}>(), {a}, {b})"));
                }
                Term::Unrelation(_ty, RVK::InVar(_var_a), RVK::InVar(_var_b)) => {
                    unimplemented!("!Rel(*a,*b) is not implemented")
                }
                Term::Unrelation(_ty, RVK::AnyVar, RVK::AnyVar) => {
                    panic!("!Rel(_,_) does not make sense.")
                }
            };
            buffer.clear();
        } else {
            match next {
                None => break,
                Some(other) => buffer.push(other),
            }
        }
    }

    assert_eq!(unequals.len(), 0);
    assert_eq!(unrelations.len(), 0);

    let mut result = String::new();

    result.push_str("{");

    generate_invar_captures(&mut result, &prefills);

    write!(
        &mut result,
        "
let world: &World = &{world};
let bk = &world.bookkeeping;
"
    );

    let mut vars: Vec<_> = variables.variables.into_values().collect();
    vars.sort();
    let infos = generate_archetype_sets(
        &mut result,
        &vars,
        &prefills,
        &components,
        &relations,
        &uncomponents,
    );
    generate_fsm_context(&mut result, &vars, &components, &relations);
    generate_resumable_query_closure(&mut result, &vars, &infos, &relations, &accessors);

    write!(
        &mut result,
        "
}}"
    );

    //eprintln!("{}", &output);
    Ok(result.parse().unwrap())
}

/// the parser treats the identifier _ of anyvars as normal variable names
/// this function changes those to the proper anyvar enum variant
fn transform_anyvars(input: Term) -> Term {
    match input {
        Term::Relation(ty, var_a, var_b) => {
            Term::Relation(ty, transform_var(var_a), transform_var(var_b))
        }
        Term::Unrelation(ty, var_a, var_b) => {
            Term::Unrelation(ty, transform_var(var_a), transform_var(var_b))
        }
        no_transform => no_transform,
    }
}

fn transform_var(input: RVK) -> RVK {
    match input {
        RVK::Var(ref var) => {
            if var == "_" {
                RVK::AnyVar
            } else {
                input
            }
        }
        RVK::InVar(ref var) => {
            if var == "_" {
                panic!("An Invar may not also be an AnyVar!\n*_ is not allowed!");
            }
            input
        }
        RVK::AnyVar => input,
    }
}
