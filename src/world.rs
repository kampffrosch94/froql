use std::{any::TypeId, collections::HashMap};

use crate::component::ComponentId;

pub struct World {
    components: HashMap<TypeId, ComponentId>,
}
