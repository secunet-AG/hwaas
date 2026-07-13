// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    serialize::ToSql,
    sql_types::Binary,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Representation of a Context Id that can be serialized to
/// and deserialized from bytes.
///
/// This is necessary for database backends that don't have
/// a native Uuid type such as Sqlite.
#[derive(
    Debug,
    Default,
    Clone,
    Serialize,
    Deserialize,
    Copy,
    PartialEq,
    Eq,
    Hash,
    FromSqlRow,
    AsExpression,
)]
#[diesel(sql_type = Binary)]
#[serde(transparent)]
pub struct ContextIdBytes(Uuid);

impl std::fmt::Display for ContextIdBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Uuid as std::fmt::Display>::fmt(&self.0, f)
    }
}

impl From<ContextIdBytes> for Uuid {
    fn from(value: ContextIdBytes) -> Self {
        value.0
    }
}

impl From<Uuid> for ContextIdBytes {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl<DB> FromSql<Binary, DB> for ContextIdBytes
where
    DB: Backend,
    Vec<u8>: FromSql<Binary, DB>,
{
    fn from_sql(
        bytes: <DB as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let bytes = Vec::<u8>::from_sql(bytes)?;
        Uuid::from_slice(&bytes)
            .map(ContextIdBytes)
            .map_err(Into::into)
    }
}

impl<DB> ToSql<Binary, DB> for ContextIdBytes
where
    DB: Backend,
    [u8]: ToSql<Binary, DB>,
{
    /// Custom implementation: Setting the value in the bindcollector directly.
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.as_bytes().as_slice().to_sql(out)
    }
}
