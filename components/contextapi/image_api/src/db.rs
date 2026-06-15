//! # Image Handler Database Interactions
//!
//! This modules defines data types and utility functions to facilitate interaction between the
//! [`ImageHandler`] and the backing database that holds information about the actual images. Most
//! prominently, this module defines the [`ID`] type, which wraps arbitrary database ID types (i.e.
//! primary key columns) in a way that makes them more ergonomic and type safe to handle.
//!
//! [ImageHandler]: crate::ImageHandler

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Generic database ID type.
///
/// Compared to a bare database ID, usually represented as a plain integer, this structure carries
/// additional information around for improved semantics. This ensures you cannot accidentally mix
/// up one object ID for another object kind, and it allows generating prettier string
/// representations, for example.
///
/// The most common way to use this type is by means of a "concrete" type alias, such as
/// [`ImageTagId`].
#[derive(
    Debug, Clone, Default, Serialize, Deserialize, JsonSchema, diesel::deserialize::FromSqlRow,
)]
pub struct ID<T: 'static, U: 'static> {
    /// Raw unique ID used to address an object in the database.
    ///
    /// This represents the tables primary key. The option is used to distinguish between objects
    /// that exist in the database (`Some`) and objects that must be created in the database
    /// (`None`).
    #[serde(skip_deserializing)]
    raw: Option<T>,
    /// Phantom use of the owned datatype `U` to distinguish type instances.
    #[serde(skip)]
    _inner: PhantomData<U>,
}

impl<T: std::cmp::PartialEq, U> std::cmp::PartialEq for ID<T, U> {
    fn eq(&self, other: &Self) -> bool {
        std::cmp::PartialEq::eq(&self.raw, &other.raw)
    }
}

impl<T: std::cmp::Eq, U> std::cmp::Eq for ID<T, U> {}

impl<T: std::hash::Hash, U> std::hash::Hash for ID<T, U> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T: std::fmt::Display, U> std::fmt::Display for ID<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name = std::any::type_name::<U>();
        if let Some(ref id) = self.raw {
            write!(f, "{:?} ID {}", type_name, id)
        } else {
            write!(f, "{:?} without ID", type_name)
        }
    }
}

impl<T, U, V, DB> diesel::deserialize::FromSql<V, DB> for ID<T, U>
where
    T: diesel::deserialize::FromSql<V, DB>,
    // NOTE: This is basically the primary key column, it *must* not be null.
    V: diesel::sql_types::SqlType<IsNull = diesel::sql_types::is_nullable::NotNull>,
    U: std::fmt::Debug,
    DB: diesel::backend::Backend,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let id = T::from_sql(bytes)?;
        Ok(Self::new(id))
    }
}

impl<T, U, V, DB> diesel::serialize::ToSql<V, DB> for ID<T, U>
where
    T: diesel::serialize::ToSql<V, DB>,
    V: diesel::sql_types::SqlType<IsNull = diesel::sql_types::is_nullable::NotNull>,
    U: std::fmt::Debug,
    DB: diesel::backend::Backend,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        let value = self
            .raw
            .as_ref()
            .expect("entities should have a concrete ID when serializing to database");

        <T as diesel::serialize::ToSql<V, DB>>::to_sql(&value, out)
    }
}

// NOTE: The underlying ID type is actually nullable (`Option`) but the API ensures that the option
// is always `Some` when propagating it to the database in operations that need it.
impl<T, U> diesel::sql_types::SqlType for ID<T, U> {
    type IsNull = diesel::sql_types::is_nullable::NotNull;
}

impl<T, U> diesel::sql_types::SingleValue for ID<T, U> {}

//impl<U, V> diesel::expression::AsExpression<V> for ID<i32, U>
//where
//    V: diesel::expression::Expression,
//{
//    type Expression = diesel::sql_types::Integer;
//
//    fn as_expression(self) -> Self::Expression {
//        &diesel::sql_types::Integer
//    }
//}

//impl<T, U> Expression for Bound<T, U>
//where
//    T: SqlType + TypedExpressionType,
//{
//    type SqlType = T;
//}

impl<U> diesel::expression::Expression for ID<i32, U> {
    type SqlType = diesel::sql_types::Integer;
}

impl<T, U> ID<T, U> {
    /// Create a new instance of an ID with a concrete value.
    ///
    /// This represents resources that exist in the database and have a unique ID.
    fn new(value: T) -> Self {
        Self {
            raw: Some(value),
            _inner: PhantomData,
        }
    }

    /// Create a new instance of an ID without a value.
    ///
    /// This represents resources that should be created in the database and don't have an ID yet.
    pub(crate) fn new_empty() -> Self {
        Self {
            raw: None,
            _inner: PhantomData,
        }
    }

    /// Check if the ID already exists in the database.
    ///
    /// The basic assumption is that an instance of `ID` with an actual ID value can only be created
    /// by reading it from the database. All other ways to create an instance of `ID` are
    /// prohibited.
    pub fn exists(&self) -> bool {
        self.raw.is_some()
    }

    /// Get the raw database ID for this instance, if any.
    ///
    /// The result can only be `Some` if this `ID` instance was obtained by reading from the
    /// database. Any ID instance created by other means *does not* have an ID.
    pub fn get(&self) -> Option<&T> {
        self.raw.as_ref()
    }
}
