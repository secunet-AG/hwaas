// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    single_context_api::context_management::ContextInfo,
    tests::test_server_setup::{TestServerOutputs, TestServerSetup},
    API_VERSION,
};
use axum::http::StatusCode;
use chrono::Utc;
use context_data_structures::aliases::ContextId;
use context_data_structures::machine_properties::MachineProperties;
use db_interaction::models::context_id::ContextIdBytes;
use diesel::{ExpressionMethods, RunQueryDsl};
use rand::{Rng, SeedableRng};
use serde_json::{json, Value};
use tracing_test::traced_test;
use wiremock::{matchers::any, Mock, Request, ResponseTemplate};

/// Obtain a context from the server satisfying the given
/// resource setup description (`rsd`).
pub(super) async fn reserve_context(addr: SocketAddr, rsd: Value) -> Result<ContextId, StatusCode> {
    let url = format!("http://{addr}/{API_VERSION}/contexts");

    let response = reqwest::Client::new()
        .post(url)
        .json(&rsd)
        .send()
        .await
        .unwrap();
    if response.status().is_success() {
        Ok(ContextId::try_parse(&response.text().await.unwrap()).unwrap())
    } else {
        Err(response.status())
    }
}

// Like reserve_context, but optimized for reusing a previously established HTTP connection.
pub(super) async fn reserve_context_with_client(
    addr: SocketAddr,
    rsd: Value,
    client: &reqwest::Client,
) -> Result<ContextId, StatusCode> {
    let url = format!("http://{addr}/{API_VERSION}/contexts");

    let response = client.post(url).json(&rsd).send().await.unwrap();
    if response.status().is_success() {
        Ok(ContextId::try_parse(&response.text().await.unwrap()).unwrap())
    } else {
        Err(response.status())
    }
}

async fn delete_context(addr: SocketAddr, ctx_id: ContextId) -> StatusCode {
    delete_context_with_client(addr, ctx_id, &reqwest::Client::new()).await
}

// Like delete_context, but optimized for reusing a previously established HTTP connection.
async fn delete_context_with_client(
    addr: SocketAddr,
    ctx_id: ContextId,
    client: &reqwest::Client,
) -> StatusCode {
    let url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}");
    client.delete(&url).send().await.unwrap().status()
}

#[derive(Debug)]
enum GetContextInfoError {
    RequestError(StatusCode),
    DeserializationError(String),
}

/// Returns information about the context if it exists
async fn get_context_info(
    addr: SocketAddr,
    ctx_id: ContextId,
) -> Result<ContextInfo, GetContextInfoError> {
    let url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}");

    let response = reqwest::Client::new().get(url).send().await.unwrap();
    let status = response.status();
    if !status.is_success() {
        return Err(GetContextInfoError::RequestError(status));
    }
    response
        .json()
        .await
        .map_err(|e| format!("{e}"))
        .map_err(GetContextInfoError::DeserializationError)
}

/// Returns the number of seconds remaining of the context lifetime.
async fn get_context_lifetime(addr: SocketAddr, ctx_id: ContextId) -> Result<u64, String> {
    match get_context_info(addr, ctx_id).await {
        Ok(info) => Ok(info.lifetime),
        Err(GetContextInfoError::RequestError(status)) => Err(format!("HTTP {}", status)),
        Err(GetContextInfoError::DeserializationError(error_string)) => Err(error_string),
    }
}

async fn extend_context_lifetime(
    addr: SocketAddr,
    ctx_id: ContextId,
    increment: u64,
) -> Result<(), StatusCode> {
    let url = format!("http://{addr}/{API_VERSION}/contexts/{ctx_id}");
    let context_patch = json!({
    "lifetime": increment,
    });

    let response = reqwest::Client::new()
        .patch(url)
        .json(&context_patch)
        .send()
        .await
        .unwrap();
    if response.status().is_success() {
        Ok(())
    } else {
        Err(response.status())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn successful_context_reservation() {
    let platform = String::from("foo");
    let server_setup = TestServerSetup::with_machine(MachineProperties {
        platform: platform.clone(),
    });
    let TestServerOutputs {
        test_db: _test_db,
        addr,
        net_ctrl_mock: _net_ctrl_mock,
        remote_mocks: _remote_mocks,
        ..
    } = server_setup.start().await;

    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "platform": platform
                }
            }
        }
    );

    assert!(reserve_context(addr, rsd).await.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
#[traced_test]
async fn context_deletion_works() {
    let platform = String::from("somePlatform");

    let outputs = TestServerSetup::with_machine(MachineProperties {
        platform: platform.clone(),
    })
    .start()
    .await;
    let rsd = json!({
        "machines": {
            "abmr": {
                "platform": platform
            }
        }
    });

    let addr = outputs.addr;

    let ctx_id = reserve_context(addr, rsd.clone()).await.unwrap();

    // Attempting to the reserve the machine again should fail
    assert!(reserve_context(addr, rsd.clone()).await.is_err());

    // Delete the context
    assert!(delete_context(addr, ctx_id).await.is_success());

    // The reservation of the context should now soon be possible
    while reserve_context(addr, rsd.clone()).await.is_err() {
        // This loop should terminate as soon as the machine is no
        // longer in maintenance mode
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn context_reservation_errors_on_empty_rsd() {
    let TestServerOutputs { addr, .. } = TestServerSetup::default().start().await;

    let rsd = json!({
        "machines": {}
    });

    let error_status = reserve_context(addr, rsd).await.unwrap_err();
    assert_eq!(error_status, StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread")]
async fn errors_on_missing_platform() {
    let platform = String::from("somePlatform");
    let other_platform = String::from("someOtherPlatfrom");
    // Configure the server to have access to one machine with platform = `platform`.
    let server_setup = TestServerSetup::with_machine(MachineProperties { platform });
    let TestServerOutputs {
        test_db: _test_db,
        addr,
        net_ctrl_mock: _net_ctrl_mock,
        remote_mocks: _remote_mocks,
        ..
    } = server_setup.start().await;

    // Ask for a context with a machine of platform = `other_platform`.
    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "platform": other_platform
                }
            }
        }
    );

    let error_status = reserve_context(addr, rsd).await.unwrap_err();
    assert_eq!(error_status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test(flavor = "multi_thread")]
async fn errors_on_already_reserved_platform() {
    let platform = String::from("foo");
    let server_setup = TestServerSetup::with_machine(MachineProperties {
        platform: platform.clone(),
    });

    // Start the server where the machine has already been reserved for some context.
    let (
        TestServerOutputs {
            test_db: _test_db,
            addr,
            net_ctrl_mock: _net_ctrl_mock,
            remote_mocks: _remote_mocks,
            ..
        },
        ..,
    ) = server_setup.start_reserved().await;

    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "platform": platform
                }
            }
        }
    );

    let error_status = reserve_context(addr, rsd).await.unwrap_err();
    assert_eq!(error_status, StatusCode::UNPROCESSABLE_ENTITY);
}

// Similar to `errors_on_already_reserved_platform`, but with multiple
// machines where one machine is already reserved, but the other is not.
#[tokio::test(flavor = "multi_thread")]
async fn errors_on_already_reserved_platform_multiple_machines() {
    let platform = String::from("someIsolatedPlatform");
    let other_platform = String::from("someOtherIsolatedPlatfrom");
    let server_setup = TestServerSetup::with_machine(MachineProperties {
        platform: platform.clone(),
    })
    .append_machine(MachineProperties {
        platform: other_platform.clone(),
    });
    let TestServerOutputs {
        test_db: _test_db,
        addr,
        net_ctrl_mock: _net_ctrl_mock,
        remote_mocks: _remote_mocks,
        ..
    } = server_setup.start().await;

    // Reserve the machine with `platform`
    assert!(reserve_context(
        addr,
        json!({
            "machines": {
                "abmr": {
                    "platform": platform.clone()
                }
            }
        })
    )
    .await
    .is_ok());

    // Now attempt to reserve two machines: One of type `platform`
    // and the other of type `other_platform`. Only the latter is free
    // hence this should fail!
    let rsd = json!({
        "machines": {
            "abmr1": {
                "platform": other_platform.clone()
            },
            "abmr2": {
                "platform": platform.clone()
            }
        }
    });
    let error_status = reserve_context(addr, rsd).await.unwrap_err();
    assert_eq!(error_status, StatusCode::UNPROCESSABLE_ENTITY);

    // Check that `other_platform` can still be reserved.
    assert!(reserve_context(
        addr,
        json!({
            "machines": {
                "abmr": {
                    "platform": other_platform
                }
            }
        })
    )
    .await
    .is_ok());
}

// This is a more involved test that exercises
// reserving and freeing the same set of machines
// repeatedly. The point here is to assert the following
// invariants:
//
// Given that there is a single set of machines
// satisfying the provided RSD `R` then the following must hold:
// 1. Two calls to reserve a context with the RSD `R` cannot succeed
// (provided that no other requests have been issued to the server in between).
//
// 2. If we know that the set of machines satisfying the RSD `R` is not reserved for
// a context, then it should be possible to reserve it for a context after a finite
// amount of attempts.
//
// 3. If an existing context containing the single set of machines compatible
// with the RSD `R` is deleted, then it should be possible to reserve `R` for
// a new context after a finite amount of attempts.
//
// Point 1. Can be specified further: If the reservation call for the RSD `R` succeeds
// then sending the same request again should yield status code: Unprocessable entity.
#[tokio::test(flavor = "multi_thread")]
#[traced_test]
async fn context_reservation_invariants() {
    // The time a context creation attempt may last before it times out.
    let context_creation_timeout = Duration::from_millis(40);

    let rng = Arc::new(Mutex::new(rand::rngs::StdRng::seed_from_u64(42)));

    // We create a server, with a slightly flaky net controller, consisting of three machines; two of `platform1`,
    // and one of `platform2`. All three have slightly flaky remote-hands.
    let platform1 = String::from("firstPlatform");
    let platform2 = String::from("secondPlatform");

    let test_server_outputs = {
        let test_server_outputs = TestServerSetup::with_machine(MachineProperties {
            platform: platform1.clone(),
        })
        .append_machine(MachineProperties {
            platform: platform1.clone(),
        })
        .append_machine(MachineProperties {
            platform: platform2.clone(),
        })
        .context_management_timeout(context_creation_timeout.as_millis() as u64)
        .no_default_net_ctrl_mock()
        .no_default_remote_mocks()
        .start()
        .await;

        let TestServerOutputs {
            net_ctrl_mock,
            remote_mocks,
            ..
        } = &test_server_outputs;

        let rng_clone = rng.clone();
        // Declare 1 in 50 net control requests to fail and
        // make 1 in 100 timeout.
        net_ctrl_mock
            .register(Mock::given(any()).respond_with(move |_req: &Request| {
                let mut guard = rng_clone.lock().unwrap();
                if guard.gen_ratio(1, 100) {
                    std::thread::sleep(context_creation_timeout + Duration::from_millis(5));
                }

                if guard.gen_ratio(1, 50) {
                    ResponseTemplate::new(500)
                } else {
                    ResponseTemplate::new(200)
                }
            }))
            .await;

        // Declare 1 in 100 remote requests to fail and make 1 in 200 timeout
        for remote_mock in remote_mocks {
            let rng = rng.clone();
            remote_mock
                .register(Mock::given(any()).respond_with(move |req: &Request| {
                    if rng.lock().unwrap().gen_ratio(1, 100) {
                        return ResponseTemplate::new(500);
                    };
                    if rng.lock().unwrap().gen_ratio(1, 200) {
                        std::thread::sleep(context_creation_timeout + Duration::from_millis(5));
                    }
                    if req.url.as_str().ends_with("/auxiliaries")
                        && req.method == wiremock::http::Method::GET
                    {
                        ResponseTemplate::new(200).set_body_json(json!([]))
                    } else {
                        ResponseTemplate::new(200)
                    }
                }))
                .await;
        }
        test_server_outputs
    };

    // Run many experiments

    // We track the id of the context that currently holds
    // all machines
    let mut ctx_id: Option<ContextId> = None;

    // RSD corresponding to all machines on the server
    let rsd = json!({
        "machines": {
            "abmr1": {
                "platform": platform1.clone()
            },
            "abmr2": {
                "platform": platform1
            },
            "abmr3": {
                "platform": platform2
            }
        }
    });

    let client = reqwest::Client::new();

    let addr = test_server_outputs.addr;

    for _ in 0..1000 {
        // Attempt to reserve a context with the given rsd. If this succeeds we have all the machines the server has access to.
        let reservation_result = reserve_context_with_client(addr, rsd.clone(), &client).await;
        if ctx_id.is_some() {
            // In this case the reservation attempt must fail and we expect an unprocessable entity status code.
            // If we allowed more complicated RSDs it would also be reasonable to
            // expect a potential timeout here, but in this test we assume timeouts cannot occur.
            assert_eq!(
                reservation_result.unwrap_err(),
                StatusCode::UNPROCESSABLE_ENTITY
            );
        }
        if ctx_id.is_none() {
            match reservation_result {
                Ok(new_ctx_id) => ctx_id = Some(new_ctx_id),
                Err(_) => {
                    // Since there is no context we conclude that some
                    // machine must be temporarily unavailable. The server
                    // should take care of making available itself, so we
                    // repeat attempting to reserve our context until we
                    // succeed.
                    while ctx_id.is_none() {
                        if let Ok(new_ctx_id) =
                            reserve_context_with_client(addr, rsd.clone(), &client).await
                        {
                            ctx_id = Some(new_ctx_id)
                        };
                    }
                }
            }
        }

        // Continue to the next round if there is no context
        let Some(current_ctx_id) = ctx_id else {
            continue;
        };

        // Flip a coin whether to delete the context or not
        if rng.lock().unwrap().gen_bool(0.5) {
            let code = delete_context_with_client(addr, current_ctx_id, &client).await;
            assert!(code != StatusCode::NOT_FOUND);
            ctx_id = None;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn context_lifetime() {
    // Increase these seconds if the test becomes flaky depending
    // on CPU resources, and whether log output is produced.
    const CONTEXT_LIFETIME: u64 = 5;

    let platform = String::from("foo");
    let server_setup = TestServerSetup::with_machine(MachineProperties {
        platform: platform.clone(),
    })
    .context_lifetime(CONTEXT_LIFETIME);
    let TestServerOutputs {
        test_db: _test_db,
        addr,
        net_ctrl_mock: _net_ctrl_mock,
        remote_mocks: _remote_mocks,
        ..
    } = server_setup.start().await;

    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "platform": platform
                }
            },
        }
    );

    let ctx_id = reserve_context(addr, rsd.clone()).await.unwrap();

    let lifetime = get_context_lifetime(addr, ctx_id).await.unwrap();
    assert!(lifetime > 0);

    // Attempting to the reserve the machine again should fail
    assert!(reserve_context(addr, rsd.clone()).await.is_err());

    // Now wait for the timeout to happen
    tokio::time::sleep(Duration::from_secs(2 * CONTEXT_LIFETIME)).await;

    // Loop until reserving the machine again works
    while reserve_context(addr, rsd.clone()).await.is_err() {
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
#[traced_test]
async fn context_lifetime_extend() {
    // Increase these seconds if the test becomes flaky depending
    // on CPU resources, and whether log output is produced.
    const CONTEXT_LIFETIME: u64 = 5;

    let platform = String::from("foo");
    let server_setup = TestServerSetup::with_machine(MachineProperties {
        platform: platform.clone(),
    })
    .context_lifetime(CONTEXT_LIFETIME);
    let TestServerOutputs {
        test_db: _test_db,
        addr,
        net_ctrl_mock: _net_ctrl_mock,
        remote_mocks: _remote_mocks,
        ..
    } = server_setup.start().await;

    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "platform": platform
                }
            },
        }
    );

    let ctx_id = reserve_context(addr, rsd.clone()).await.unwrap();
    let lifetime = get_context_lifetime(addr, ctx_id).await.unwrap();
    assert!(lifetime > 0);
    // Attempting to the reserve the machine again should fail
    assert!(reserve_context(addr, rsd.clone()).await.is_err());

    extend_context_lifetime(addr, ctx_id, 10 * CONTEXT_LIFETIME)
        .await
        .unwrap();

    // Now wait for the initial timeout to happen
    tokio::time::sleep(Duration::from_secs(2 * CONTEXT_LIFETIME)).await;

    let lifetime = get_context_lifetime(addr, ctx_id).await.unwrap();
    assert!(lifetime > 0);

    // Attempting to the reserve the machine again should still fail
    assert!(reserve_context(addr, rsd.clone()).await.is_err());
}

// Tests that it is possible to extend the lifetime of a context
// directly via the database without going through the REST api.
//
// More precisely it checks that this keeps the context alive
// for the duration specified in the database and the machine(s)
// assigned to the context may not be reserved for other contexts
// within this time interval as expected.
//
// The test also ensures that once the timeout set in the database
// has passed the context gets deleted (as usual) and the machines
// that were previously assigned to it are now free to be reserved
// for new contexts again.
//
// NOTE: Unlike the vast majority of tests this does not test
// something users can do, but is a test for maintainers. The
// use of this functionality is expected to be on a rare ad-hoc
// basis, but we still want to know that it works as expected.
#[tokio::test(flavor = "multi_thread")]
#[traced_test]
async fn extend_context_lifetime_via_db() {
    // Startup a test server and reserve a context with a lifetime of four seconds where
    // contexts may not have lifetimes above 5 seconds.
    let context_start_lifetime = 4;
    let context_max_lifetime_configuration = 5;
    // The platform of the only machine available to the spawned Context API
    let platform = String::from("foo");
    let (mut outputs, ctx_id) = TestServerSetup::with_machine(MachineProperties {
        platform: platform.clone(),
    })
    .context_lifetime(context_start_lifetime)
    .context_max_lifetime(context_max_lifetime_configuration)
    .start_reserved()
    .await;

    // Reality check there is at least 2 seconds remaining of the context lifetime when we
    // request it and also that it is indeed less than 5 seconds which is the configured
    // maximum.
    let lifetime = get_context_lifetime(outputs.addr, ctx_id).await.unwrap();
    assert!(lifetime > (context_start_lifetime - 2));
    assert!(lifetime < context_max_lifetime_configuration);

    // Change the context lifetime in the database directly
    let seconds_more_than_max = 4;
    let new_context_lifetime = context_max_lifetime_configuration + seconds_more_than_max;
    {
        // Database related code goes in this block
        let timeout = Utc::now() + Duration::from_secs(new_context_lifetime);
        let timeout = timeout.naive_utc();
        use db_interaction::schema::context_lifetimes as context_lifetimes_schema;
        diesel::update(context_lifetimes_schema::table)
            .filter(context_lifetimes_schema::context_id.eq(ContextIdBytes::from(ctx_id)))
            .set(context_lifetimes_schema::timeout.eq(timeout))
            .execute(&mut outputs.test_db.conn)
            .expect("should be able to execute database update");
    };
    // Retrieve the timeout and check that it exceeds the configured maximum. This
    // works because we bypassed the server checks by writing directly to the
    // database which is considered the source of truth
    let lifetime = get_context_lifetime(outputs.addr, ctx_id).await.unwrap();

    assert!(lifetime > context_max_lifetime_configuration);
    // Spawn a task that sleeps for context_max_lifetime_configuration then
    // requests the remaining context lifetime and checks that it is greater than
    // zero. It also checks that the one machine known to the Context API (reserved
    // for the context) may not be reserved at this point.
    let addr = outputs.addr;
    let rsd = json!({"machines": { "abmr": { "platform": platform } } });
    let rsd_clone = rsd.clone();
    let wait_max_then_check_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(context_max_lifetime_configuration)).await;
        // We now get the context lifetime and attempt to reserve the reserved machine for another context. These
        // attempts must complete before the context times out as we want to assert the expected behaviour when the
        // context is still active.
        let must_complete_within = Duration::from_secs(seconds_more_than_max - 2);
        let remaining_lifetime_fut =
            tokio::time::timeout(must_complete_within, get_context_lifetime(addr, ctx_id));
        let machine_reservation_fut =
            tokio::time::timeout(must_complete_within, reserve_context(addr, rsd_clone));

        let (remaining_lifetime, machine_reservation_attempt) =
            tokio::join!(remaining_lifetime_fut, machine_reservation_fut);

        // If the test fails we want to see errors here to check if timeouts happened.
        dbg!(&remaining_lifetime);
        dbg!(&machine_reservation_attempt);
        let remaining_lifetime = remaining_lifetime.unwrap().unwrap();
        let status = machine_reservation_attempt
            .expect("Should not have timed out")
            .unwrap_err();
        (remaining_lifetime > 0) && (status == StatusCode::UNPROCESSABLE_ENTITY)
    });

    // Spawn a task that sleeps for a couple of seconds longer than the new context lifetime and checks that
    // the context no longer exists after this time has passed. It also checks that the machine that previously
    // belonged to the context is now free and can be reserved.
    let wait_exceeding_new_context_lifetime_task = tokio::spawn(async move {
        // Wait for 2 seconds more than new_context_lifetime. The context should have been
        // deleted within these extra two seconds.
        tokio::time::sleep(Duration::from_secs(new_context_lifetime + 2)).await;
        // The context should be gone so we should not be able to retrieve info. The status code
        // should be either NOT_FOUND or GONE
        let err = get_context_info(addr, ctx_id).await.unwrap_err();
        let context_was_not_found = match err {
            GetContextInfoError::RequestError(status) => {
                (status == StatusCode::NOT_FOUND) || (status == StatusCode::GONE)
            }
            _ => false,
        };
        let new_context_reservation = reserve_context(addr, rsd).await;
        // This can help debugging if the test should fail
        dbg!(&new_context_reservation);
        context_was_not_found && new_context_reservation.is_ok()
    });
    // Wait for the spawned tasks to finish and assert that they both return `true`.
    assert!(wait_max_then_check_task
        .await
        .expect("wait max task should not have panicked"));
    assert!(wait_exceeding_new_context_lifetime_task
        .await
        .expect("wait exceeding new context lifetime task should not have panicked"));
}

// Check that context reservation works as expected when we
// permit machine ids in the RSD.
//
// What this test ensures:
// - When an id is specified then if the context reservation succeeds
// we know that the corresponding machine is reserved and may not be
// reserved for another context.
// - An id does not have to be used for every machine  only for the
// machine(s) where it is desirable.
#[tokio::test(flavor = "multi_thread")]
async fn context_reservation_with_machine_id() {
    let platform_foo = String::from("foo");
    let platform_bar = String::from("bar");

    let first_machine_id = 42;
    let second_machine_id = 9;
    let server_setup = TestServerSetup::with_machine_and_id(
        MachineProperties {
            platform: platform_foo.clone(),
        },
        first_machine_id,
    )
    .append_machine_with_id(
        MachineProperties {
            platform: platform_bar.clone(),
        },
        second_machine_id,
    );

    let TestServerOutputs {
        test_db: _test_db,
        addr,
        net_ctrl_mock: _net_ctrl_mock,
        remote_mocks: _remote_mocks,
        ..
    } = server_setup.start().await;

    // Reserve a context where we specify the id of the first
    // machine. The second machine has no id provided.
    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "platform": platform_foo,
                    "machine_id": first_machine_id
                },
                "abmr2": {
                    "platform": platform_bar,
                }
            }
        }
    );

    assert!(reserve_context(addr, rsd).await.is_ok());

    // Check that we may not reserve platform_foo for a new context
    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "platform": platform_foo
                }
            }
        }
    );

    assert!(reserve_context(addr, rsd).await.is_err());

    // It should also not be possible to reserve the other machine now
    let rsd = json!({
        "machines": {
            "abmr": {
                "platform": platform_bar,
                "machine_id": second_machine_id
            }
        }
    });

    assert!(reserve_context(addr, rsd).await.is_err());
}

// Check that one may not reserve a machine by specifying its id
// if one specifies an incorrect platform.
#[tokio::test(flavor = "multi_thread")]
async fn context_reservation_with_machine_id_wrong_platform() {
    let platform_foo = String::from("foo");

    let machine_id = 42;
    let server_setup = TestServerSetup::with_machine_and_id(
        MachineProperties {
            platform: platform_foo.clone(),
        },
        machine_id,
    );

    let TestServerOutputs {
        test_db: _test_db,
        addr,
        net_ctrl_mock: _net_ctrl_mock,
        remote_mocks: _remote_mocks,
        ..
    } = server_setup.start().await;

    // Reserve a context where we specify an existing id
    // but a non-existing platform.
    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "platform": format!("not {}", platform_foo),
                    "machine_id": machine_id
                },
            }
        }
    );

    assert_eq!(
        reserve_context(addr, rsd).await.unwrap_err(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

// Check that an attempt to reserve a machine with an id
// that does not exist will fail.
#[tokio::test(flavor = "multi_thread")]
async fn context_reservation_machine_id_does_not_exist() {
    let platform_foo = String::from("foo");

    let machine_id = 42;
    let server_setup = TestServerSetup::with_machine_and_id(
        MachineProperties {
            platform: platform_foo.clone(),
        },
        machine_id,
    );

    let TestServerOutputs {
        test_db: _test_db,
        addr,
        net_ctrl_mock: _net_ctrl_mock,
        remote_mocks: _remote_mocks,
        ..
    } = server_setup.start().await;

    // Reserve a context where we specify a machine with an existing
    // platform, but non-existing id.
    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "platform": platform_foo,
                    "machine_id": machine_id - 5
                },
            }
        }
    );

    assert_eq!(
        reserve_context(addr, rsd).await.unwrap_err(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

// Check that one may not reserve a machine by specifying its id
// if one does not specify a platform at all.
#[tokio::test(flavor = "multi_thread")]
async fn context_reservation_with_machine_id_no_platform() {
    let platform_foo = String::from("foo");

    let machine_id = 42;
    let server_setup = TestServerSetup::with_machine_and_id(
        MachineProperties {
            platform: platform_foo.clone(),
        },
        machine_id,
    );

    let TestServerOutputs {
        test_db: _test_db,
        addr,
        net_ctrl_mock: _net_ctrl_mock,
        remote_mocks: _remote_mocks,
        ..
    } = server_setup.start().await;

    // Reserve a context where we specify only an id.
    let rsd = json!(
        {
            "machines": {
                "abmr": {
                    "machine_id": machine_id
                },
            }
        }
    );

    assert_eq!(
        reserve_context(addr, rsd).await.unwrap_err(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
}
