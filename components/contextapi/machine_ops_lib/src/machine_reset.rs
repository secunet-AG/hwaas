// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{future::Future, sync::Arc, time::Duration};

use db_interaction::{
    connection::DbFacade,
    models::{
        aliases::MachineId,
        machines::{Machine, SwitchConnection, SwitchPort},
    },
    schema::enabled_ports as enabled_ports_schema,
    schema::machines as machines_schema,
    schema::switch_ports as switch_ports_schema,
};

use diesel::prelude::*;
use error_utils::log_err;
use net_ctrl_client_wrapper::NetCtrlClient;
use remote_client::RemoteClient;
use tokio::task::JoinSet;
use tracing::{debug, error, info_span, instrument, warn, Instrument, Span};

use crate::machine_data::{
    InvalidRemoteAuxiliaryBaseUrl, InvalidRemotePowerBaseUrl, InvalidRemoteSerialBaseUrl,
    InvalidRemoteUsbBaseUrl, RemoteAuxiliaryBaseUrl, RemotePowerBaseUrl, RemoteSerialBaseUrl,
    RemoteUsbBaseUrl,
};

/// Maximum duration to sleep in between retries when force-retrying fallibly operations until
/// success.
const MAXIMUM_RETRY_SLEEP_DURATION: Duration = Duration::from_mins(10);

/// Data necessary for resetting a given machine.
struct MachineResetData {
    machine: Machine,
    switch_ports: Vec<SwitchPort>,
}
/// Resets the machine.
///
/// The following actions are performed until they succeed:
/// - Power is turned off
/// - All switch ports associated with the machine are deactivated
/// - All serial buffers associated with the machine are reset
/// - All auxiliary devices connected to the machine are deconfigured
/// - The usb device connected to the machine gets deconfigured
///
/// # Warning: Caller responsibilities
///
/// ## Machine state transitions
///
/// This function does **not** alter the `state` column in the
/// `machines` database table at all.
///
/// The caller is thus responsible for updating the machine state
/// to a suitable value prior to and after calling this function.
///
/// We place this burden on the caller because (1) a reset may be
/// associated with more states and (2) the caller may want to perform
/// some more work before setting the machine state to `Free` after
/// calling this function.
///
/// Indeed the `Initializing` state may not be left before the
/// machine has been successfully reset and it is beneficial to
/// not have `Resetting` as a follow up state from `Initializing`
/// as we then loose information regarding the reason for the reset.
///
/// ## Retries
///
/// This function returns a future that _might never resolve_. All
/// retryable actions are indeed retried (with exponential backoff)
/// until success. Hence the caller should wrap the future in a
/// timeout before awaiting it.
///
/// # Hard errors
///
/// This function requires URIs to be obtained in order to contact
/// external services governing certain functionality (such as the machine's power for instance).
///
/// If the required URIs loaded from the database happen to be malformed an error will be returned
/// immediately before any observable side effects take place.
///
/// # Testing
///
/// This function is extensively tested in the [`initialization`](crate::initialization)
/// module. We test it as part of the machine initialization tests as the function
/// requires a lot of non-trivial setup.
#[instrument(skip(db_facade, net_ctrl_client, remote_client))]
pub async fn reset_machine(
    machine_id: MachineId,
    db_facade: Arc<DbFacade>,
    net_ctrl_client: NetCtrlClient,
    remote_client: RemoteClient,
) -> Result<(), IrrecoverableMachineResetError> {
    // Load machine data from the database
    let read_machine_data_gen = || {
        async {
            let current_span_id = Span::current().id();

            let machine_data: Option<MachineResetData> = db_facade
                .spawn_call(move |conn| {
                    // Give this database read attempt its own span. Note that as the
                    // error is bubbled up we don't want to over report it, hence we
                    // don't include error data obtained from Diesel in the events reported
                    // within this span.
                    let span = info_span!("machine_data_read_attempt", %machine_id);
                    span.follows_from(current_span_id);

                    let _guard = span.entered();
                    let machine: Option<Machine> = machines_schema::table
                        .find(machine_id)
                        .first(conn)
                        .optional()
                        .inspect_err(|_| error!("machine lookup failed"))?;

                    let Some(machine) = machine else {
                        return Ok(None);
                    };

                    let switch_ports: Vec<SwitchPort> = SwitchConnection::belonging_to(&machine)
                        .inner_join(switch_ports_schema::table)
                        .select(SwitchPort::as_select())
                        .load(conn)?;

                    Ok(Some(MachineResetData {
                        machine,
                        switch_ports,
                    }))
                })
                .await
                .inspect_err(log_err!("failed to read machine data"))?;
            Result::<_, db_interaction::connection::DatabaseInteractionError>::Ok(machine_data)
        }
    };

    let span = info_span!("machine_data_read");
    let Some(MachineResetData {
        machine,
        switch_ports,
    }): Option<MachineResetData> =
        retry_until_success(read_machine_data_gen, Some(MAXIMUM_RETRY_SLEEP_DURATION))
            .instrument(span)
            .await
    else {
        // The machine was not found which we warn against, but still treat as
        // a success.
        warn!("machine not found in the database. Maybe it has been deleted?");
        return Ok(());
    };

    let Machine {
        id,
        remote_usb,
        remote_power,
        remote_serial,
        remote_auxiliary,
        ..
    } = machine;

    // Reality check when running in debug builds (e.g. for testing)
    debug_assert_eq!(id, machine_id);

    // Parse addresses to the Uri type before invoking remote services.
    let remote_power = RemotePowerBaseUrl::try_from(remote_power.clone()).map_err(|e| {
        IrrecoverableMachineResetError::CouldNotParseRemotePowerUri {
            address: remote_power,
            error: e,
        }
    })?;
    let usb_address = RemoteUsbBaseUrl::try_from(remote_usb.clone()).map_err(|e| {
        IrrecoverableMachineResetError::CouldNotParseRemoteUsbUri {
            address: remote_usb,
            error: e,
        }
    })?;

    let auxiliary_address = remote_auxiliary
        .clone()
        .map(RemoteAuxiliaryBaseUrl::try_from)
        .transpose()
        .map_err(
            |e| IrrecoverableMachineResetError::CouldNotParseRemoteAuxUri {
                address: remote_auxiliary,
                error: e,
            },
        )?;

    let serial_address = remote_serial
        .clone()
        .map(RemoteSerialBaseUrl::try_from)
        .transpose()
        .map_err(
            |e| IrrecoverableMachineResetError::CouldNotParseRemoteSerialUri {
                address: remote_serial,
                error: e,
            },
        )?;
    // Power off the machine
    {
        debug!("going to power off machine");
        let span = info_span!("power_off_machine");
        let client = remote_client.clone();
        let power_off = move || {
            let client = client.clone();
            let remote_power = remote_power.clone();
            async move {
                let response = remote_power
                    .apply_reset(&client)
                    .await
                    .map_err(|e| error!(error.dbg=?e, "failed to power off the machine"))?;
                let status = response.status();
                if !status.is_success() {
                    error!(%status, "request to power off machine did not return a successful status");
                    Err(())
                } else {
                    Ok(())
                }
            }
        };
        retry_until_success(power_off, Some(MAXIMUM_RETRY_SLEEP_DURATION))
            .instrument(span.or_current())
            .await;
        debug!("the machine was successfully powered off");
    }

    // The following tasks require no database modifications hence we run them concurrently
    tokio::join!(
        reset_serial_buffers(machine_id, serial_address, remote_client.clone()),
        deactivate_aux_devices(machine_id, auxiliary_address, remote_client.clone()),
        deconfigure_usb(machine_id, usb_address, remote_client.clone())
    );

    // This task will write to the database
    disconnect_switch_ports(switch_ports, db_facade, net_ctrl_client).await;
    Ok(())
}

#[instrument(skip_all)]
async fn reset_serial_buffers(
    machine_id: MachineId,
    serial_address: Option<RemoteSerialBaseUrl>,
    remote_client: RemoteClient,
) {
    let Some(serial_address) = serial_address else {
        // If there is no serial address then we are already done.
        return;
    };

    // We create a closure that returns a future that attempts to reset the serial buffers managed by the given service.
    // Since we want to be able to call it more than once and that the returned future should have a
    // 'static lifetime (so it may be spawned) we need to clone the arguments at the beginning of each call.
    let buffer_reset = move || {
        let client = remote_client.clone();
        let serial_address = serial_address.clone();
        async move {
            let response = serial_address.clone().apply_reset(&client).await.inspect_err(|e| error!(error.dbg = ?e, %serial_address, %machine_id, "remote serial reset request failed")).map_err(|_|())?;
            let status = response.status();
            if !status.is_success() {
                error!(%status, %serial_address, %machine_id, "got an unsuccessful status when attempting to reset serial buffer");
                Err(())
            } else {
                debug!(%serial_address, %machine_id, "successfully reset serial buffer");
                Ok(())
            }
        }
    };

    retry_until_success(buffer_reset, Some(MAXIMUM_RETRY_SLEEP_DURATION)).await;
}

/// Deactivate all aux devices connected to the machine. This function retries until it succeeds with
/// exponential backoff.
async fn deactivate_aux_devices(
    machine_id: MachineId,
    remote_auxiliary_address: Option<RemoteAuxiliaryBaseUrl>,
    remote_client: RemoteClient,
) {
    let Some(remote_auxiliary_address) = remote_auxiliary_address else {
        // If there is no auxiliary address then we are already done.
        return;
    };
    // We create a closure that returns a future that attempts to reset the auxiliary devices managed by the given service.
    // Since we want to be able to call it more than once and that the returned future should have a
    // 'static lifetime (so it may be spawned) we need to clone the arguments at the beginning of each call.
    let reset_auxiliaries = move || {
        let client = remote_client.clone();
        let address = remote_auxiliary_address.clone();
        async move {
            let response = address.clone().apply_reset(&client).await.inspect_err(|e| error!(error.dbg = ?e, %address, %machine_id, "remote auxiliary reset request failed")).map_err(|_|())?;
            let status = response.status();
            if !status.is_success() {
                error!(%status, %address, "request to deactivate aux devices did not return a success status code");
                Err(())
            } else {
                debug!(device_address = %address, "successfully deactivated aux devices");
                Ok(())
            }
        }
    };
    retry_until_success(reset_auxiliaries, Some(MAXIMUM_RETRY_SLEEP_DURATION)).await;
}

#[instrument(skip_all)]
async fn disconnect_switch_ports(
    switch_ports: Vec<SwitchPort>,
    db_facade: Arc<DbFacade>,
    net_ctrl_client: NetCtrlClient,
) {
    let mut join_set = JoinSet::new();

    let port_ids: Vec<i32> = switch_ports
        .iter()
        .map(|switch_port| switch_port.id)
        .collect();

    debug!(port_database_ids = ?port_ids, "going to disconnect switch ports");
    for switch_port in switch_ports {
        let client = net_ctrl_client.clone();
        let disable_port = move || {
            let switch_port = switch_port.clone();
            let client = client.clone();
            let span = info_span!("disable_switch_port_attempt", ?switch_port);
            async move {
                client
                    .disable_port(&switch_port)
                    .await
                    .inspect_err(log_err!("failed to disable switch port"))
                    .inspect(|_| debug!("successfully disabled switch port"))
            }
            .instrument(span)
        };
        join_set.spawn(
            retry_until_success(disable_port, Some(MAXIMUM_RETRY_SLEEP_DURATION)).in_current_span(),
        );
    }

    // Wait until all spawned tasks have completed
    while let Some(join_result) = join_set.join_next().await {
        if join_result.is_err() {
            let _ = join_result.inspect_err(log_err!("unexpected join error"));
            // Nothing we can do here so we pend and let the caller handle the resulting timeout
            return std::future::pending().await;
        }
    }

    // Make sure to update the database accordingly
    let port_ids = Arc::new(port_ids);
    debug!("going to update database: deleting enabled switch ports as they are now disabled");
    let delete_enabled_ports = move || {
        let port_ids = port_ids.clone();
        let db = db_facade.clone();
        async move {
            // Execute this on the current thread as we do not want the write to occur after this
            // task has been dropped
            db.execute_on_current_thread(move |conn| {
                conn.immediate_transaction(|conn| {
                    diesel::delete(enabled_ports_schema::table)
                        .filter(enabled_ports_schema::id.eq_any(port_ids.as_ref()))
                        .execute(conn)
                        .inspect_err(log_err!("unable to delete enabled ports"))?;
                    Ok(())
                })
            })
            .await
        }
    };
    retry_until_success(delete_enabled_ports, Some(MAXIMUM_RETRY_SLEEP_DURATION))
        .in_current_span()
        .await;
}

/// Deconfigures the usb connected to the machine. This will retry until it succeeds with increasingly
/// long sleeps between retries.
#[instrument(skip_all)]
async fn deconfigure_usb(
    machine_id: MachineId,
    usb_address: RemoteUsbBaseUrl,
    remote_client: RemoteClient,
) {
    // Create a closure for the reset request that we pass on to `retry_until_success`
    let client = remote_client.clone();
    let reset = move || {
        let client = client.clone();
        let usb_address = usb_address.clone();
        async move {
            let response = usb_address
                .clone()
                .apply_reset(&client)
                .await
                .map_err(|e| error!(error=?e, "failed to reset usb"))?;
            let status = response.status();
            if !status.is_success() {
                error!(%status, %usb_address, %machine_id, "received unsuccessful status code when attempting to deconfigure usb");
                Err(())
            } else {
                debug!(%usb_address, %machine_id, "successfully deconfigured usb");
                Ok(())
            }
        }
    };
    retry_until_success(reset, Some(MAXIMUM_RETRY_SLEEP_DURATION)).await;
}

/// Takes a closure that returns a future with a fallible outcome which is then called
/// and resolved until it succeeds. There are increasingly long sleeps between retries
/// (exponential backoff). You can configure the maximum time to sleep between retries with
/// `max_sleep` which, if given, will never be exceeded (i.e. it "limits" the exponential backoff).
#[instrument(skip(f))]
async fn retry_until_success<FutGen, Fut, Err, T>(f: FutGen, max_sleep: Option<Duration>) -> T
where
    FutGen: Fn() -> Fut,
    Fut: Future<Output = Result<T, Err>>,
{
    let mut backoff_exponent = 1;
    loop {
        if let Ok(output) = f().await {
            return output;
        } else {
            let raw_backoff = 6_u64.saturating_pow(backoff_exponent);
            let backoff = Duration::from_millis(raw_backoff);
            let sleep_duration = if let Some(value) = max_sleep {
                std::cmp::min(value, backoff)
            } else {
                backoff
            };
            debug!(milliseconds = %sleep_duration.as_millis(), "sleeping before retrying");

            tokio::time::sleep(sleep_duration).await;
            backoff_exponent += 1;
        }
    }
}

#[derive(Debug)]
pub enum IrrecoverableMachineResetError {
    CouldNotParseRemoteSerialUri {
        address: Option<String>,
        error: InvalidRemoteSerialBaseUrl,
    },
    CouldNotParseRemoteAuxUri {
        address: Option<String>,
        error: InvalidRemoteAuxiliaryBaseUrl,
    },
    CouldNotParseRemoteUsbUri {
        address: String,
        error: InvalidRemoteUsbBaseUrl,
    },
    CouldNotParseRemotePowerUri {
        address: String,
        error: InvalidRemotePowerBaseUrl,
    },
}

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    #[tokio::test]
    async fn retry_respects_backoff_limit() {
        let counter = Arc::new(AtomicUsize::new(100));
        let slow = move || {
            let inner_counter = counter.clone();
            async move {
                let count = inner_counter.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                if count > 0 {
                    Err("Nope")
                } else {
                    Ok(())
                }
            }
        };

        let started = tokio::time::Instant::now();
        retry_until_success(slow, Some(Duration::from_millis(1))).await;
        let ended = started.elapsed();
        assert!(ended > Duration::from_millis(95));
        // NOTE: In theory this can take arbitrarily long, but for practical reasons this should
        // suffice I hope.
        assert!(ended < Duration::from_secs(1));
    }
}
