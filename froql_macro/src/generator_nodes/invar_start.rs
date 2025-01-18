use super::GeneratorNode;
use std::fmt::Write;
use std::ops::Range;

#[derive(Debug)]
pub struct InvarStart {
    /// index of the component of `old` where the relation resides
    pub unequalities: Vec<(isize, isize)>,
}

impl GeneratorNode for InvarStart {
    fn generate(&self, step: usize, _prepend: &mut String, append: &mut String) -> usize {
        if self.unequalities.is_empty() {
            write!(
                append,
                r#"
0 => {{
    return None;
}}
"#
            )
            .unwrap();
        } else {
            write!(
                append,
                r#"
0 => {{
    todo!("Check for unrelations.");
    return None;
}}
"#
            )
            .unwrap();
        }

        return step + 1;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn invar_trivial() {
        let gen = InvarStart {
            unequalities: vec![],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(0, &mut prepend, &mut append);
        assert_eq!(1, r);
        insta::assert_snapshot!(prepend, @"");
        insta::assert_snapshot!(append, @r#"
        0 => {
            return None;
        }
        "#);
    }

    #[test]
    #[ignore = "WIP"]
    fn invar_unequality() {
        let gen = InvarStart {
            unequalities: vec![(0, 2), (2, 1)],
        };

        let mut prepend = String::new();
        let mut append = String::new();
        let r = gen.generate(0, &mut prepend, &mut append);
        assert_eq!(2, r);
        insta::assert_snapshot!(prepend, @"");
        insta::assert_snapshot!(append);
    }
}
