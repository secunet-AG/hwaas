// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use super::context_id::ContextIdBytes;

pub(crate) mod private {}

/// Corresponds to the contexts table.
#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Serialize,
    Deserialize,
)]
#[diesel(table_name = crate::schema::contexts)]
pub struct ContextIdentifier {
    pub id: ContextIdBytes,
}

impl From<ContextIdBytes> for ContextIdentifier {
    fn from(value: ContextIdBytes) -> Self {
        Self { id: value }
    }
}

impl From<ContextIdentifier> for ContextIdBytes {
    fn from(value: ContextIdentifier) -> Self {
        value.id
    }
}

/// Corresponds to the context_lifetimes table.
///
/// Note at the moment every context has a lifetime
/// hence we could consider inlining this table.
#[derive(
    Debug,
    Clone,
    Copy,
    Insertable,
    Queryable,
    Selectable,
    Serialize,
    Deserialize,
    Identifiable,
    Associations,
)]
#[diesel(table_name = crate::schema::context_lifetimes)]
#[diesel(belongs_to(ContextIdentifier, foreign_key = context_id))]
#[diesel(primary_key(context_id))]
pub struct ContextLifetime {
    /// The id of the corresponding context.
    pub context_id: ContextIdBytes,
    /// When the context was created.
    pub created: NaiveDateTime,
    /// When the context is set to timeout.
    pub timeout: NaiveDateTime,
}
