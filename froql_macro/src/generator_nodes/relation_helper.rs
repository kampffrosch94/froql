use std::fmt::Write;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RelationHelperInfo {
    /// Index where the relationship cid is in the cid array for `old_var`
    pub cid_index: usize,
    pub column_index: usize,
    pub old_var: isize,
    pub new_var: isize,
    /// nr of this RelationHelper, used when generating variable name
    pub nr: usize,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UnrelationHelperInfo {
    /// Index where the relationship cid is in the cid array for `old_var`
    pub ty: String,
    pub flip_target: bool,
    pub old_var: isize,
    pub new_var: isize,
    /// nr of this RelationHelper, used when generating variable name
    pub nr: usize,
}

pub fn relation_helpers_init_and_set_col(
    prepend: &mut String,
    append: &mut String,
    helpers: &[RelationHelperInfo],
    unhelpers: &[UnrelationHelperInfo],
) {
    for helper in helpers {
        let old = helper.old_var;
        let nr = helper.nr;
        let column_index = helper.column_index;
        let cid_index = helper.cid_index;
        write!(
            prepend,
            "
let mut rel_helper_{nr} = ::froql::query_helper::RelationHelper::new
    (components_{old}[{cid_index}]);
"
        )
        .unwrap();
        write!(
            append,
            "
    rel_helper_{nr}.set_col(&a_ref.columns[col_indexes[{column_index}]]);
"
        )
        .unwrap();
    }

    for unhelper in unhelpers {
        let old = unhelper.old_var;
        let nr = unhelper.nr;
        if unhelper.flip_target {
            write!(prepend, "").unwrap();
            todo!();
        } else {
            write!(prepend, "").unwrap();
            todo!();
        }
    }
}

pub fn relation_helpers_set_rows(
    append: &mut String,
    helpers: &[RelationHelperInfo],
    unhelpers: &[UnrelationHelperInfo],
) {
    for helper in helpers {
        let nr = helper.nr;
        let var = helper.old_var;
        write!(
            append,
            "
        rel_helper_{nr}.set_row(bk, a_rows[{var}].0);
"
        )
        .unwrap();
    }
}
