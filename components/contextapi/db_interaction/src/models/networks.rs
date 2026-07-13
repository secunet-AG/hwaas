// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use diesel::prelude::*;
use serde::Deserialize;

use super::aliases::NetworkId;
use super::context_id::ContextIdBytes;
use context_data_structures::aliases::NetworkName;

/// A network identifier.
///
/// There is at most one [`Network`] per network identifier.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Hash, Insertable, Queryable, Selectable, Deserialize,
)]
#[diesel(table_name = crate::schema::network_identifiers)]
pub struct NetworkIdentifier {
    pub id: NetworkId,
}

/// A network.
///
/// The machines connected to a network can be traced by via the [`EnabledPorts`](crate::models::machines::EnabledPort) entities.
#[derive(
    Debug, Clone, Hash, Eq, PartialEq, Queryable, Selectable, Insertable, Identifiable, Associations,
)]
#[diesel(table_name = crate::schema::networks)]
#[diesel(belongs_to(crate::models::contexts::ContextIdentifier, foreign_key = context_id))]
pub struct Network {
    /// The identifier of the network and corresponding [`NetworkIdentifier`].
    pub id: NetworkId,
    /// The context the network belongs to.
    pub context_id: ContextIdBytes,
    /// User assigned network name.
    pub name: NetworkName,
}
