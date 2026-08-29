use std::{collections::HashMap, fmt, sync::Arc};

use cookbench_core::domain::HarnessId;

use crate::HarnessAdapter;

#[derive(Default)]
pub struct AdapterRegistry {
    adapters: HashMap<HarnessId, Arc<dyn HarnessAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, adapter: Arc<dyn HarnessAdapter>) -> Result<(), RegistryError> {
        let id = adapter.id();
        if self.adapters.contains_key(&id) {
            return Err(RegistryError::DuplicateAdapterId(id));
        }
        self.adapters.insert(id, adapter);
        Ok(())
    }

    pub fn get(&self, id: &HarnessId) -> Option<Arc<dyn HarnessAdapter>> {
        self.adapters.get(id).cloned()
    }

    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    DuplicateAdapterId(HarnessId),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateAdapterId(id) => write!(formatter, "adapter already registered: {id:?}"),
        }
    }
}

impl std::error::Error for RegistryError {}
