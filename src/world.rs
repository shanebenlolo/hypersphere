pub struct World {
    pub entities_count: usize,
    pub component_vecs: Vec<Box<dyn ComponentVec>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities_count: 0,
            component_vecs: Vec::new(),
        }
    }

    pub fn new_entity(&mut self) -> usize {
        let entity_id = self.entities_count;
        for component_vec in self.component_vecs.iter_mut() {
            component_vec.push_none();
        }
        self.entities_count += 1;
        entity_id
    }

    pub fn add_component_to_entity<ComponentType: 'static>(
        &mut self,
        entity: usize,
        component: ComponentType,
    ) {
        for component_vec in self.component_vecs.iter_mut() {
            if let Some(component_vec) = component_vec
                .as_any_mut()
                .downcast_mut::<Vec<Option<ComponentType>>>()
            {
                component_vec[entity] = Some(component);
                return;
            }
        }

        // No matching component storage exists yet, so we have to make one.
        let mut new_component_vec: Vec<Option<ComponentType>> =
            Vec::with_capacity(self.entities_count);

        // All existing entities don't have this component, so we give them `None`
        for _ in 0..self.entities_count {
            new_component_vec.push(None);
        }

        // Give this Entity the Component.
        new_component_vec[entity] = Some(component);
        self.component_vecs.push(Box::new(new_component_vec));
    }

    //  finds and borrows the ComponentVec that matches a type
    pub fn borrow_component_vec<ComponentType: 'static>(
        &self,
    ) -> Option<&Vec<Option<ComponentType>>> {
        for component_vec in self.component_vecs.iter() {
            if let Some(component_vec) = component_vec
                .as_any()
                .downcast_ref::<Vec<Option<ComponentType>>>()
            {
                return Some(component_vec);
            }
        }
        None
    }

    pub fn get_component<ComponentType: 'static>(&self, entity: usize) -> Option<&ComponentType> {
        if entity >= self.entities_count {
            return None;
        }

        self.borrow_component_vec::<ComponentType>()
            .and_then(|vec| vec.get(entity))
            .and_then(|option| option.as_ref())
    }

    pub fn query_entities_with_material_and_mesh<MeshType: 'static, MaterialType: 'static>(
        &self,
    ) -> Vec<usize> {
        // Get the component vectors for Mesh and Material, if they exist
        let mesh_vec = self.borrow_component_vec::<MeshType>();
        let material_vec = self.borrow_component_vec::<MaterialType>();

        // Check if both component vectors are available
        if let (Some(mesh_vec), Some(material_vec)) = (mesh_vec, material_vec) {
            // Collect the entity IDs that have both components
            let mut entities_with_both = Vec::new();
            for (entity_id, (mesh, material)) in
                mesh_vec.iter().zip(material_vec.iter()).enumerate()
            {
                if mesh.is_some() && material.is_some() {
                    entities_with_both.push(entity_id);
                }
            }
            entities_with_both
        } else {
            // If one of the component vectors is missing, return an empty Vec
            Vec::new()
        }
    }

    pub fn query_entities_with_mesh_but_no_material<MeshType: 'static, MaterialType: 'static>(
        &self,
    ) -> Vec<usize> {
        // Get the component vector for Mesh, and check if Material vector exists
        let mesh_vec = self.borrow_component_vec::<MeshType>();
        let material_vec = self.borrow_component_vec::<MaterialType>();

        match (mesh_vec, material_vec) {
            (Some(mesh_vec), Some(material_vec)) => {
                // Collect the entity IDs that have Mesh but no Material
                mesh_vec
                    .iter()
                    .enumerate()
                    .filter_map(|(entity_id, mesh)| {
                        if mesh.is_some()
                            && material_vec
                                .get(entity_id)
                                .and_then(|m| m.as_ref())
                                .is_none()
                        {
                            Some(entity_id)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            (Some(mesh_vec), None) => {
                // If there is no Material vector, return all entities with Mesh
                mesh_vec
                    .iter()
                    .enumerate()
                    .filter_map(|(entity_id, mesh)| {
                        if mesh.is_some() {
                            Some(entity_id)
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            _ => Vec::new(), // Return an empty Vec if Mesh vector doesn't exist
        }
    }
}

trait ComponentVec {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn push_none(&mut self);
}

impl<T: 'static> ComponentVec for Vec<Option<T>> {
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
    fn push_none(&mut self) {
        self.push(None)
    }
}
