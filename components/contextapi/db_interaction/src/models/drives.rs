// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use diesel::prelude::*;

use super::{aliases::DriveId, context_id::ContextIdBytes};

use context_data_structures::aliases::DriveName;

/// Drive data which may be read from the database.
#[derive(
    Debug, Clone, Queryable, Insertable, Selectable, Eq, PartialEq, Identifiable, Associations,
)]
#[diesel(table_name = crate::schema::drives)]
#[diesel(belongs_to(crate::models::contexts::ContextIdentifier, foreign_key = context_id))]
pub struct Drive {
    /// The database identifier of the drive.
    pub id: DriveId,
    /// The user defined name of the drive.
    pub name: DriveName,
    /// The context the drive belongs to.
    pub context_id: ContextIdBytes,
}
