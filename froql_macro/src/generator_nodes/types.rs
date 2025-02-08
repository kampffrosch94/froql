#[derive(Debug)]
pub struct RelationConstraint {
    pub helper_nr: usize,
    /// only necessary for invar checks
    /// when joining we always check the just joined id
    pub checked_invar: Option<isize>,
}

#[derive(Debug)]
pub struct UnrelationConstraint {
    pub helper_nr: usize,
    /// only necessary for invar checks
    /// when joining we always check the just joined id
    pub checked_invar: Option<isize>,
}
