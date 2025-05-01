use proc_macro::TokenTree;

use crate::macro_error::MacroError;

macro_rules! error {
    ($arr:expr, $($arg:tt)*) => {
        return Err(MacroError::slice($arr, format!($($arg)*)))
    };
}

macro_rules! error_single {
    ($tt:expr, $($arg:tt)*) => {
        return Err(MacroError::start_end($tt, $tt, format!($($arg)*)))
    };
}

pub enum VarKind {
    /// variable that just exists in the query
    Var(String),
    /// variable that is coming into the query from an outside scope
    /// marked with a * before its name in the query! syntax
    InVar(String),
}
use VarKind as VK;

pub enum RelationVarKind {
    /// variable that just exists in the query
    Var(String),
    /// variable that is coming into the query from an outside scope
    /// marked with a * before its name in the query! syntax
    InVar(String),
    /// just a _
    AnyVar,
}
use RelationVarKind as RVK;

pub enum Term {
    /// Type, Variable
    ComponentVar(String, VarKind),
    /// Type, Variable
    MutComponentVar(String, VarKind),
    /// Type, Variable
    /// but this Component won't be in the output tuple
    NoOutComponentVar(String, VarKind),
    /// Type, Variable, Variable
    Relation(String, RelationVarKind, RelationVarKind),
    /// VariableName
    OutVar(String),
    /// VarA, VarB
    ConstraintUnequal(VarKind, VarKind),
    /// Type, VariableName
    Uncomponent(String, String),
    /// Type, Variable, Variable
    Unrelation(String, RelationVarKind, RelationVarKind),
    /// Type, VariableName
    OptionalComponent(String, String),
    /// Type, VariableName
    OptionalMutComponent(String, String),
}

pub fn parse_term(tokens: &[TokenTree]) -> Result<Term, MacroError> {
    use TokenTree as TT;
    if tokens.len() == 1 {
        return match &tokens[0] {
            TT::Ident(ty) => Ok(Term::ComponentVar(
                ty.to_string(),
                VK::Var("this".to_string()),
            )),
            x => error!(tokens, "Expected Component, got {x:?}"),
        };
    }
    if tokens.len() == 2 {
        return match (&tokens[0], &tokens[1]) {
            (TT::Ident(mutability), TT::Ident(ty)) => match mutability.to_string().as_str() {
                "mut" => {
                    return Ok(Term::MutComponentVar(
                        ty.to_string(),
                        VK::Var("this".to_string()),
                    ));
                }
                "_" => {
                    return Ok(Term::NoOutComponentVar(
                        ty.to_string(),
                        VK::Var("this".to_string()),
                    ));
                }
                _ => {
                    error_single!(&tokens[0], "Expected mut or _");
                }
            },
            x @ (TT::Ident(ty), TT::Group(group)) => {
                let ty = ty.to_string();
                let mut iter = group.stream().into_iter();
                let first = iter
                    .next()
                    .expect("Expected type for Component or Relation");
                let second = iter.next(); // TODO inline
                let third = iter.next();
                let fourth = iter.next();
                let fifth = iter.next();
                match (&first, &second, &third, &fourth, &fifth) {
                    (TT::Ident(var), None, None, None, None) => {
                        Ok(Term::ComponentVar(ty, VK::Var(var.to_string())))
                    }
                    (start_tt @ TT::Punct(star), Some(TT::Ident(var)), None, None, None) => {
                        if star.as_char() != '*' {
                            error_single!(start_tt, "Expected '*'");
                        }
                        Ok(Term::ComponentVar(ty, VK::InVar(var.to_string())))
                    }
                    (
                        TT::Ident(rel_a),
                        Some(TT::Punct(comma)),
                        Some(TT::Ident(rel_b)),
                        None,
                        None,
                    ) => {
                        if comma.as_char() != ',' {
                            error_single!(second.as_ref().unwrap(), "Expected ','");
                        }
                        Ok(Term::Relation(
                            ty,
                            RVK::Var(rel_a.to_string()),
                            RVK::Var(rel_b.to_string()),
                        ))
                    }
                    (
                        TT::Punct(star),
                        Some(TT::Ident(rel_a)),
                        Some(TT::Punct(comma)),
                        Some(TT::Ident(rel_b)),
                        None,
                    ) => {
                        if star.as_char() != '*' {
                            error_single!(&first, "Expected '*'");
                        }
                        if comma.as_char() != ',' {
                            error_single!(third.as_ref().unwrap(), "Expected ','");
                        }
                        Ok(Term::Relation(
                            ty,
                            RVK::InVar(rel_a.to_string()),
                            RVK::Var(rel_b.to_string()),
                        ))
                    }
                    (
                        TT::Ident(rel_a),
                        Some(TT::Punct(comma)),
                        Some(TT::Punct(star)),
                        Some(TT::Ident(rel_b)),
                        None,
                    ) => {
                        if star.as_char() != '*' {
                            error_single!(third.as_ref().unwrap(), "Expected '*'");
                        }
                        if comma.as_char() != ',' {
                            error_single!(second.as_ref().unwrap(), "Expected ','");
                        }
                        Ok(Term::Relation(
                            ty,
                            RVK::Var(rel_a.to_string()),
                            RVK::InVar(rel_b.to_string()),
                        ))
                    }

                    (
                        TT::Punct(star),
                        Some(TT::Ident(rel_a)),
                        Some(TT::Punct(comma)),
                        Some(TT::Punct(star2)),
                        Some(TT::Ident(rel_b)),
                    ) => {
                        if star.as_char() != '*' {
                            error_single!(&first, "Expected '*'");
                        }
                        if star2.as_char() != '*' {
                            error_single!(fourth.as_ref().unwrap(), "Expected '*'");
                        }
                        if comma.as_char() != ',' {
                            error_single!(third.as_ref().unwrap(), "Expected ','");
                        }
                        Ok(Term::Relation(
                            ty,
                            RVK::InVar(rel_a.to_string()),
                            RVK::InVar(rel_b.to_string()),
                        ))
                    }
                    _ => error!(
                        tokens,
                        "expected Component(var) or Relation(a,b), got {x:?}"
                    ),
                }
            }
            (TT::Punct(punct), TT::Ident(ident)) => match punct.as_char() {
                '&' => return Ok(Term::OutVar(ident.to_string())),
                '!' => return Ok(Term::Uncomponent(ident.to_string(), "this".to_string())),
                _ => error_single!(&tokens[0], "Expected & or !"),
            },
            // Example: CompA?
            (TT::Ident(ty), t_question @ TT::Punct(question)) => {
                if question.as_char() != '?' {
                    error_single!(t_question, "Expected '?'");
                }
                return Ok(Term::OptionalComponent(ty.to_string(), "this".to_string()));
            }
            x => {
                error!(
                    tokens,
                    "expected mut Component or Component(var) or Relation(a,b) or &var or Component?, got {x:?}"
                );
            }
        };
    }
    if tokens.len() == 3 {
        match (&tokens[0], &tokens[1], &tokens[2]) {
            x @ (TT::Ident(mut_or_), TT::Ident(ty), TT::Group(group)) => {
                let mut iter = group.stream().into_iter();
                match (iter.next(), iter.next()) {
                    (Some(TT::Ident(var)), None) => match mut_or_.to_string().as_str() {
                        "mut" => {
                            return Ok(Term::MutComponentVar(
                                ty.to_string(),
                                VK::Var(var.to_string()),
                            ));
                        }
                        "_" => {
                            return Ok(Term::NoOutComponentVar(
                                ty.to_string(),
                                VK::Var(var.to_string()),
                            ));
                        }
                        _ => {
                            error_single!(&tokens[0], "Expected mut or _");
                        }
                    },
                    (Some(ref start_tt @ TT::Punct(ref star)), Some(TT::Ident(var))) => {
                        if star.as_char() != '*' {
                            error_single!(start_tt, "Expected '*'");
                        }
                        match mut_or_.to_string().as_str() {
                            "mut" => {
                                return Ok(Term::MutComponentVar(
                                    ty.to_string(),
                                    VK::InVar(var.to_string()),
                                ));
                            }
                            "_" => {
                                return Ok(Term::NoOutComponentVar(
                                    ty.to_string(),
                                    VK::InVar(var.to_string()),
                                ));
                            }
                            _ => {
                                error_single!(&tokens[0], "Expected mut or _");
                            }
                        }
                    }
                    _ => error!(
                        tokens,
                        "expected <mut|_> Component(var), \n got {x:?} {}",
                        line!()
                    ),
                }
            }
            // mut Comp?
            (t_mut @ TT::Ident(mut_), TT::Ident(ty), t_question @ TT::Punct(question)) => {
                if mut_.to_string().as_str() != "mut" {
                    error_single!(t_mut, "Expected mut");
                }
                if question.as_char() != '?' {
                    error_single!(t_question, "Expected '?'");
                }
                return Ok(Term::OptionalMutComponent(
                    ty.to_string(),
                    "this".to_string(),
                ));
            }
            (ref bang_t @ TT::Punct(bang), TT::Ident(ty), TT::Group(group)) => {
                match bang.as_char() {
                    '!' => (),
                    _ => error_single!(&bang_t, "Expected !"),
                };

                let mut iter = group.stream().into_iter();
                match (iter.next(), iter.next(), iter.next(), iter.next()) {
                    (Some(TT::Ident(var)), None, None, None) => {
                        return Ok(Term::Uncomponent(ty.to_string(), var.to_string()));
                    }
                    (
                        Some(TT::Ident(var_a)),
                        Some(ref t_comma @ TT::Punct(ref comma)),
                        Some(TT::Ident(var_b)),
                        None,
                    ) => {
                        if comma.as_char() != ',' {
                            error_single!(t_comma, "Expected ','");
                        }
                        return Ok(Term::Unrelation(
                            ty.to_string(),
                            RVK::Var(var_a.to_string()),
                            RVK::Var(var_b.to_string()),
                        ));
                    }
                    (
                        Some(ref t_star @ TT::Punct(ref star)),
                        Some(TT::Ident(var_a)),
                        Some(ref t_comma @ TT::Punct(ref comma)),
                        Some(TT::Ident(var_b)),
                    ) => {
                        if comma.as_char() != ',' {
                            error_single!(t_comma, "Expected ','");
                        }
                        if star.as_char() != '*' {
                            error_single!(t_star, "Expected '*'");
                        }
                        return Ok(Term::Unrelation(
                            ty.to_string(),
                            RVK::InVar(var_a.to_string()),
                            RVK::Var(var_b.to_string()),
                        ));
                    }
                    (
                        Some(TT::Ident(var_a)),
                        Some(ref t_comma @ TT::Punct(ref comma)),
                        Some(ref t_star @ TT::Punct(ref star)),
                        Some(TT::Ident(var_b)),
                    ) => {
                        if comma.as_char() != ',' {
                            error_single!(t_comma, "Expected ','");
                        }
                        if star.as_char() != '*' {
                            error_single!(t_star, "Expected '*'");
                        }
                        return Ok(Term::Unrelation(
                            ty.to_string(),
                            RVK::Var(var_a.to_string()),
                            RVK::InVar(var_b.to_string()),
                        ));
                    }
                    group => error!(
                        tokens,
                        "expected !Component(var) or !Rel(a,b), got {group:?} {}",
                        line!()
                    ),
                }
            }
            (TT::Ident(ty), TT::Group(group), ref question_t @ TT::Punct(question)) => {
                match question.as_char() {
                    '?' => (),
                    _ => error_single!(&question_t, "Expected ?"),
                };
                let mut iter = group.stream().into_iter();
                match (iter.next(), iter.next()) {
                    (Some(TT::Ident(ident)), None) => {
                        return Ok(Term::OptionalComponent(ty.to_string(), ident.to_string()));
                    }
                    group => error!(tokens, "expected Component(var)? got {group:?} {}", line!()),
                }
            }
            _ => {
                error!(
                    tokens,
                    "expected <mut|_|!> Component<(var)|?>, got {tokens:?} {}",
                    line!()
                )
            }
        };
    }

    if tokens.len() == 4 {
        match (&tokens[0], &tokens[1], &tokens[2], &tokens[3]) {
            // var_a != var_b
            (TT::Ident(id_a), TT::Punct(bang), TT::Punct(equal), TT::Ident(id_b)) => {
                let var_a = id_a.to_string();
                let var_b = id_b.to_string();
                match (bang.as_char(), equal.as_char()) {
                    ('!', '=') => (),
                    _ => error!(&tokens[3..4], "Expected var_a != var_b"),
                }
                return Ok(Term::ConstraintUnequal(VK::Var(var_a), VK::Var(var_b)));
            }
            // mut Comp(var)?
            (
                t_mut @ TT::Ident(mut_),
                TT::Ident(ty),
                t_group @ TT::Group(group),
                t_question @ TT::Punct(question),
            ) => {
                if mut_.to_string().as_str() != "mut" {
                    error_single!(t_mut, "Expected mut");
                }
                if question.as_char() != '?' {
                    error_single!(t_question, "Expected '?'");
                }

                let mut iter = group.stream().into_iter();
                match (iter.next(), iter.next()) {
                    (Some(TT::Ident(var)), None) => {
                        return Ok(Term::OptionalMutComponent(ty.to_string(), var.to_string()));
                    }
                    _ => {
                        error_single!(t_group, "Expected var got: {t_group:?}");
                    }
                }
            }
            _ => {
                error!(tokens, "Expected var_a != var_b, got: {tokens:?}");
            }
        }
    }
    if tokens.len() == 5 {
        match (&tokens[0], &tokens[1], &tokens[2], &tokens[3], &tokens[4]) {
            (
                TT::Ident(id_a),
                TT::Punct(bang),
                TT::Punct(equal),
                t_star @ TT::Punct(star),
                TT::Ident(id_b),
            ) => {
                let var_a = id_a.to_string();
                let var_b = id_b.to_string();
                match (bang.as_char(), equal.as_char()) {
                    ('!', '=') => (),
                    _ => error!(&tokens[3..4], "Expected var_a != var_b"),
                }
                if star.as_char() != '*' {
                    error_single!(t_star, "Expected '*'");
                }
                return Ok(Term::ConstraintUnequal(VK::Var(var_a), VK::InVar(var_b)));
            }
            (
                t_star @ TT::Punct(star),
                TT::Ident(id_a),
                TT::Punct(bang),
                TT::Punct(equal),
                TT::Ident(id_b),
            ) => {
                let var_a = id_a.to_string();
                let var_b = id_b.to_string();
                match (bang.as_char(), equal.as_char()) {
                    ('!', '=') => (),
                    _ => error!(&tokens[3..4], "Expected var_a != var_b"),
                }
                if star.as_char() != '*' {
                    error_single!(t_star, "Expected '*'");
                }
                return Ok(Term::ConstraintUnequal(VK::InVar(var_a), VK::Var(var_b)));
            }
            _ => error!(tokens, "Expected: a != *b or *a != b but got: {tokens:?}"),
        }
    }
    if tokens.len() == 6 {
        match (
            &tokens[0], &tokens[1], &tokens[2], &tokens[3], &tokens[4], &tokens[5],
        ) {
            (
                t_star @ TT::Punct(star),
                TT::Ident(id_a),
                TT::Punct(bang),
                TT::Punct(equal),
                t_star2 @ TT::Punct(star2),
                TT::Ident(id_b),
            ) => {
                let var_a = id_a.to_string();
                let var_b = id_b.to_string();
                match (bang.as_char(), equal.as_char()) {
                    ('!', '=') => (),
                    _ => error!(&tokens[3..4], "Expected var_a != var_b"),
                }
                if star.as_char() != '*' {
                    error_single!(t_star, "Expected '*'");
                }
                if star2.as_char() != '*' {
                    error_single!(t_star2, "Expected '*'");
                }
                return Ok(Term::ConstraintUnequal(VK::InVar(var_a), VK::InVar(var_b)));
            }
            _ => error!(tokens, "Expected: *a != *b but got: {tokens:?}"),
        }
    }

    let len = tokens.len();
    error!(tokens, "Can't parse this: Len: {len} {tokens:?}");
}
