use std::marker::PhantomData;

use uuid::Uuid;

/// Entity ID
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId<T> {
    inner: Uuid,
    _phantom: PhantomData<T>,
}

impl<T> Clone for EntityId<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _phantom: self._phantom.clone(),
        }
    }
}

impl<T> EntityId<T> {
    pub fn new() -> Self {
        Self {
            inner: Uuid::now_v7(),
            _phantom: PhantomData,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.inner
    }
}

impl<T> From<Uuid> for EntityId<T> {
    fn from(value: Uuid) -> Self {
        EntityId {
            inner: value,
            _phantom: PhantomData,
        }
    }
}
