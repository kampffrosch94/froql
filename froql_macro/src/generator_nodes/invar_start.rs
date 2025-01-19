use super::{relation_join::insert_checks, GeneratorNode};
use std::fmt::Write;

#[derive(Debug)]
pub struct InvarStart {
    /// index of the component of `old` where the relation resides
    pub unequalities: Vec<(isize, isize)>,
    pub rel_constraints: Vec<(usize, isize, isize)>,
}

impl GeneratorNode for InvarStart {
    fn generate(&self, step: usize, prepend: &mut String, append: &mut String) -> usize {
        if self.unequalities.is_empty() && self.rel_constraints.is_empty() {
            assert_eq!(step, 0);
            write!(
                append,
                r#"
{step} => {{
    return None;
}}
"#
            )
            .unwrap();
            // because 0 is our exit we have to start at 1
            prepend.push_str(
                "
current_step = 1;",
            );
            return step + 1;
        } else {
            write!(
                append,
                r#"
{step} => {{
"#
            )
            .unwrap();
            insert_checks(append, &self.unequalities, &self.rel_constraints);
            append.push_str(
                "
    {
        return None;
    } else {
        current_step += 2;
    }
}",
            );
            let next_step = step + 1;
            // end state
            write!(
                append,
                r#"
{next_step} => {{
    return None;
}}
"#
            )
            .unwrap();
            return step + 2;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn invar_trivial() {
        let gen = InvarStart {
            unequalities: vec![],
            rel_constraints: vec![],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(0, &mut prepend, &mut append);
        assert_eq!(1, r);
        insta::assert_snapshot!(prepend, @"current_step = 1;");
        insta::assert_snapshot!(append, @r#"
        0 => {
            return None;
        }
        "#);
    }

    #[test]
    fn invar_unequality() {
        let gen = InvarStart {
            unequalities: vec![(0, 2), (2, 1)],
            rel_constraints: vec![],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(0, &mut prepend, &mut append);
        assert_eq!(2, r);
        insta::assert_snapshot!(prepend, @"");
        insta::assert_snapshot!(append, @r#"
        0 => {

                    if
                        (::std::ptr::eq(a_refs[0], a_refs[2])
                         && a_rows[0] == a_rows[2])
                    ||
                        (::std::ptr::eq(a_refs[2], a_refs[1])
                         && a_rows[2] == a_rows[1])
            {
                return None;
            } else {
                current_step += 2;
            }
        }
        1 => {
            return None;
        }
        "#);
    }
}
