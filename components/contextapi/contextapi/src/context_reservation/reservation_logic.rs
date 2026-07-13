// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use context_data_structures::{
    aliases::MachineNameStr,
    rsd::{ResourceConstraints, Rsd},
};
use db_interaction::models::machines::MachineReservation;
use db_interaction::models::machines::MachineState as ModelMachineState;
use db_interaction::models::{context_id::ContextIdBytes, machines::Machine as ModelMachine};

/// The reason why the RSD could not be resolved.
///
/// All variants wrap a `machine_id` field describing the identifier of the
/// desired machine first established not being able to satisfy the specified
/// constraints described in the RSD.
pub(super) enum RsdResolutionError<'rsd> {
    /// There is no free compatible machine, but there exists at least
    /// one reserved machine satisfying the constraints.
    NoFreeCompatibleMachine { machine_id: &'rsd MachineNameStr },
    /// There is no machine satisfying the given constraints regardless of its
    /// reservation status.
    NoCompatibleMachine { machine_id: &'rsd MachineNameStr },
}

/// Resolves the given `rsd` and returns a vector of machine reservations upon success.
///
/// If one or more machines described in the RSD does not match a free compatible machine in the
/// store an error is returned.
///
/// Note: This function does not touch the database.
pub(super) fn resolve_rsd(
    mut machines: Vec<ModelMachine>,
    rsd: &Rsd,
    context_id: ContextIdBytes,
) -> Result<Vec<MachineReservation>, RsdResolutionError<'_>> {
    let mut provisional_machines: Vec<MachineReservation> = Vec::new();
    for (machine_id, constraints) in rsd.machines.iter() {
        // Try to find a machine that satisfies the constraints
        let mut compatible_machine_exists = false;
        let compatible_machine: &mut ModelMachine = machines
            .iter_mut()
            .find_map(|machine| {
                let ResourceConstraints {
                    platform,
                    machine_id,
                } = constraints;
                // The platforms must match, even if a unique
                // machine_id is set.
                if (platform == &machine.platform)
                    && machine_id
                        .or(Some(machine.id))
                        .filter(|&id| id == machine.id)
                        .is_some()
                {
                    compatible_machine_exists = true;
                    (machine.state == ModelMachineState::Free).then_some(machine)
                } else {
                    None
                }
            })
            .ok_or(if compatible_machine_exists {
                RsdResolutionError::NoFreeCompatibleMachine { machine_id }
            } else {
                RsdResolutionError::NoCompatibleMachine { machine_id }
            })?;
        // Mark the machine as reserved so it does not get picked again in a later loop iteration
        compatible_machine.state = ModelMachineState::Reserved;
        provisional_machines.push(MachineReservation {
            id: compatible_machine.id,
            context_id,
            machine_name: machine_id.clone(),
        });
    }
    Ok(provisional_machines)
}
