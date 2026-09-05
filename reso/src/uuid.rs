use std::marker::PhantomData;

use rusqlite::{ToSql, types::FromSql};
use uuid::Uuid;

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
        *self
    }
}

impl<T> Copy for EntityId<T> {}

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

impl<T> ToSql for EntityId<T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.inner.to_sql()
    }
}

impl<T> FromSql for EntityId<T> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let uuid = Uuid::column_result(value)?;

        rusqlite::types::FromSqlResult::Ok(EntityId::from(uuid))
    }
}
