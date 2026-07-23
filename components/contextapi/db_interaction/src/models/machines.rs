// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use chrono::NaiveDateTime;
use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    prelude::*,
    serialize::ToSql,
    sql_types::SmallInt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::FromRepr;

use super::contexts::ContextIdentifier;
use super::{
    aliases::{MachineId, RemoteAddress},
    context_id::ContextIdBytes,
};
use context_data_structures::aliases::MachineNetworkInterface;

/// Represents the state of a given [`Machine`].
#[non_exhaustive]
#[repr(i16)]
#[derive(
    FromRepr,
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Hash,
    FromSqlRow,
    AsExpression,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[diesel(sql_type = SmallInt)]
pub enum MachineState {
    /// The machine is being initialized.
    Initializing = 0,
    /// The machine is free and may be reserved for a context.
    Free = 1,
    /// The machine is reserved for a context.
    Reserved = 2,
    /// The machine is in the process of being reset.
    Resetting = 3,
    /// The machine has been deactivated. This means all of its switch ports are
    /// deactivated.
    Deactivated = 4,
}

impl From<MachineState> for i16 {
    fn from(value: MachineState) -> Self {
        value as i16
    }
}

impl<DB> ToSql<SmallInt, DB> for MachineState
where
    DB: Backend,
    i16: ToSql<SmallInt, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        match self {
            Self::Initializing => (Self::Initializing as i16).to_sql(out),
            Self::Free => (Self::Free as i16).to_sql(out),
            Self::Reserved => (Self::Reserved as i16).to_sql(out),
            Self::Resetting => (Self::Resetting as i16).to_sql(out),
            Self::Deactivated => (Self::Deactivated as i16).to_sql(out),
        }
    }
}
impl<DB> FromSql<SmallInt, DB> for MachineState
where
    DB: Backend,
    i16: FromSql<SmallInt, DB>,
{
    /// Need this to read [`MachineState`] as if it was a SmallInt in the database.
    fn from_sql(
        bytes: <DB as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let code = i16::from_sql(bytes)?;
        MachineState::from_repr(code)
            .ok_or_else(|| format!("invalid machine state representation: {}", code).into())
    }
}

/// A Machine.
///
/// Note that a machine has more associated data than the fields listed here,
/// but that data resides in other tables than `machine`.
#[derive(
    AsChangeset,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Ord,
    Serialize,
    Deserialize,
    PartialOrd,
    Hash,
    Queryable,
    Selectable,
    Insertable,
    Identifiable,
    JsonSchema,
)]
#[diesel(table_name = crate::schema::machines)]
pub struct Machine {
    /// The identifier of the machine.
    pub id: MachineId,
    /// The machine's platform.
    pub platform: String,
    /// An address for the machine's remote-usb server.
    pub remote_usb: RemoteAddress,
    /// An address for the machine's remote-power server.
    pub remote_power: RemoteAddress,
    /// An address for the machine's remote-serial server.
    pub remote_serial: Option<RemoteAddress>,
    /// An address for the machine's remote-auxiliary server.
    pub remote_auxiliary: Option<RemoteAddress>,
    /// The state of the machine.
    pub state: MachineState,
    /// An address for the machine's remote-mjpeg server.
    pub remote_mjpeg: Option<RemoteAddress>,
}

/// A machine reset corresponding to a [`Machine`].
#[derive(
    AsChangeset,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    Queryable,
    Selectable,
    Insertable,
)]
#[diesel(table_name = crate::schema::machine_resets)]
pub struct MachineReset {
    /// The id of the corresponding machine.
    pub id: MachineId,
    /// When the reset first started.
    pub started: NaiveDateTime,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    Associations,
    PartialEq,
    Hash,
    Identifiable,
    Queryable,
    Selectable,
    Insertable,
)]
#[diesel(table_name = crate::schema::machine_reservations)]
#[diesel(belongs_to(Machine, foreign_key = id))]
#[diesel(belongs_to(ContextIdentifier, foreign_key = context_id))]
pub struct MachineReservation {
    /// The id of the corresponding machine.
    pub id: MachineId,
    /// The context this reservation belongs to.
    pub context_id: ContextIdBytes,
    /// The name assigned to the machine by the context owner.
    pub machine_name: String,
}

/// The port of a switch.
#[derive(
    AsChangeset,
    Debug,
    Eq,
    PartialEq,
    Hash,
    Clone,
    Queryable,
    Selectable,
    Insertable,
    Identifiable,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[diesel(table_name = crate::schema::switch_ports)]
pub struct SwitchPort {
    /// The identifier in the database.
    pub id: i32,
    /// The name of the switch.
    pub switch: String,
    /// The port of the switch.
    pub port: String,
}

/// Represents the fact that a [`SwitchPort`] is enabled.
#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    Eq,
    PartialEq,
    Insertable,
    Identifiable,
    Associations,
    Selectable,
    Queryable,
)]
#[diesel(table_name = crate::schema::enabled_ports)]
#[diesel(belongs_to(SwitchPort, foreign_key = id))]
#[diesel(belongs_to(super::networks::Network, foreign_key = net_id))]
pub struct EnabledPort {
    /// The id of the corresponding [`SwitchPort`].
    pub id: i32,
    /// The id of the corresponding [`Network`](crate::models::networks::Network).
    pub net_id: i16,
}

/// Similar to [`SwitchPort`], but does not
/// have an identifier.
///
/// This is useful for inserting new ports into the database.
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Insertable,
    Queryable,
    Selectable,
    JsonSchema,
)]
#[diesel(table_name = crate::schema::switch_ports)]
pub struct SwitchPortValue {
    /// The name of the switch.
    pub switch: String,
    /// The port on the switch.
    pub port: String,
}

impl From<SwitchPort> for SwitchPortValue {
    fn from(SwitchPort { switch, port, .. }: SwitchPort) -> Self {
        SwitchPortValue { switch, port }
    }
}

/// A switch connection of a [`Machine`].
///
/// For each [`SwitchPort`] there is at most one corresponding
/// [`SwitchConnection`].
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Hash,
    Eq,
    PartialEq,
    JsonSchema,
    Identifiable,
    Associations,
    Queryable,
    Selectable,
    Insertable,
)]
#[diesel(table_name = crate::schema::switch_connections)]
#[diesel(belongs_to(SwitchPort, foreign_key = id))]
#[diesel(belongs_to(Machine, foreign_key = machine_id))]
pub struct SwitchConnection {
    /// The identifier in the database.
    pub id: i32,
    /// The name of the network interface (i.e. "lan1").
    pub interface: MachineNetworkInterface,
    /// The database identifier of the machine this connection belongs to.
    pub machine_id: MachineId,
}
