use std::marker::PhantomData;

use uuid::Uuid;

/// Entity ID
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId<T> {
    inner: Uuid,
    _phantom: PhantomData<T>,
}

impl<T> serde::Serialize for EntityId<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<'de, T> serde::Deserialize<'de> for EntityId<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self {
            inner: Uuid::deserialize(deserializer)?,
            _phantom: PhantomData,
        })
    }
}

impl<T> Clone for EntityId<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _phantom: self._phantom,
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
