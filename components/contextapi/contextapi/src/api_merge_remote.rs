// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::openapi::ParameterSchemaOrContent::Schema;
use aide::openapi::{
    Components, OpenApi, Operation, Parameter, ParameterData, PathItem, Paths, ReferenceOr,
    SchemaObject,
};
use aide::transform::TransformOpenApi;
use context_data_structures::aliases::{ContextId, MachineName};
use schemars::schema_for;
use serde_json::{Map, Value};
use std::fs;
use tracing::{error, warn};

fn get_remote_oas(remote_oas_path: String) -> Result<OpenApi, ()> {
    let remote_oas_file =
        fs::read_to_string(remote_oas_path).map_err(|e| error!("Could not read file: {}", e))?;

    serde_json::from_str(remote_oas_file.as_str())
        .map_err(|e| error!("Could not parse OAS: {:?}", e))
}

/// Merge json_schema values
fn merge(a: &mut Value, b: &Value) {
    match (a, b) {
        (&mut Value::Object(ref mut a), Value::Object(b)) => {
            for (k, v) in b {
                merge(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

fn transform_remote_oas(remote_oas: OpenApi) -> OpenApi {
    //The iterated items are tuples of (&str, &str, &Operation) containing the path, method, and the operation.
    let transformed_paths = remote_oas
        .operations()
        .map(|(path, methode, op)| {
            (
                format!("/contexts/{{ctx_id}}/machines/{{machine_name}}{path}"),
                methode,
                op,
            )
        })
        // Fix the operation and remove the "UUID" PathParam
        .map(|(path, methode, op)| {
            let mut op = op.clone();

            op.parameters.push(ReferenceOr::Item(Parameter::Path {
                parameter_data: ParameterData {
                    name: "ctx_id".to_string(),
                    description: Some("Context access token".to_string()),
                    required: true,
                    deprecated: None,
                    format: Schema(SchemaObject {
                        json_schema: schema_for!(ContextId).schema.into(),
                        external_docs: None,
                        example: None,
                    }),
                    example: None,
                    examples: Default::default(),
                    explode: None,
                    extensions: Default::default(),
                },
                style: Default::default(),
            }));

            op.parameters.push(ReferenceOr::Item(Parameter::Path {
                parameter_data: ParameterData {
                    name: "machine_name".to_string(),
                    description: Some("Name of the Machine".to_string()),
                    required: true,
                    deprecated: None,
                    format: Schema(SchemaObject {
                        json_schema: schemars::schema::Schema::from(
                            schema_for!(MachineName).schema,
                        ),
                        external_docs: None,
                        example: None,
                    }),
                    example: None,
                    examples: Default::default(),
                    explode: None,
                    extensions: Default::default(),
                },
                style: Default::default(),
            }));

            let op = Operation {
                tags: vec!["Machines API".to_string()],
                ..op.clone()
            };
            (path, methode, op)
        })
        // build the updated Path schema
        .fold(Paths::default(), |mut acc, (path, methode, op)| {
            let mut item = serde_json::to_value(PathItem {
                ..acc
                    .paths
                    .get_mut(&path)
                    .map(|e: &mut ReferenceOr<PathItem>| e.clone().into_item().unwrap_or_default())
                    .unwrap_or_default()
            })
            .unwrap();

            let new = Value::Object(Map::from_iter(vec![(
                methode.to_string(),
                serde_json::to_value(op).unwrap(),
            )]));
            merge(&mut item, &new);

            acc.paths
                .insert(path, serde_json::from_value(item).unwrap());

            acc
        });

    let transformed_components = remote_oas.components.map(|comps| {
        let mut transformed_schemas = comps.schemas.clone();
        transformed_schemas.swap_remove("AuxiliaryDevicePath");
        transformed_schemas.swap_remove("ControlID");
        transformed_schemas.swap_remove("SerialID");
        transformed_schemas.swap_remove("AuxiliaryID");

        Components {
            schemas: transformed_schemas,
            ..comps
        }
    });

    OpenApi {
        components: transformed_components,
        paths: Some(transformed_paths),
        ..remote_oas
    }
}

pub(crate) fn merge_remote_oas(
    remote_oas_path: String,
    mut transform: TransformOpenApi,
) -> TransformOpenApi {
    let remote_oas: OpenApi = get_remote_oas(remote_oas_path)
        .map_err(|_| {
            warn!("Could not get remote-hands OpenAPI specs - proceeding with empty default")
        })
        .unwrap_or_default();

    let remote_oas = transform_remote_oas(remote_oas);

    // Merge paths
    transform.inner_mut().paths = match (transform.inner_mut().paths.clone(), remote_oas.paths) {
        (Some(mut paths), Some(remote_paths)) => {
            paths.paths.extend(remote_paths.paths);
            paths.extensions.extend(remote_paths.extensions);
            Some(paths)
        }
        (None, Some(remote_paths)) => Some(remote_paths),
        (Some(path), None) => Some(path),
        _ => None,
    };

    // Merge schemas
    transform.inner_mut().components = match (
        transform.inner_mut().components.clone(),
        remote_oas.components,
    ) {
        (Some(mut c), Some(remote_c)) => {
            c.schemas.extend(remote_c.schemas);
            Some(c)
        }
        (Some(c), None) => Some(c),
        (None, Some(remote_c)) => Some(Components {
            schemas: remote_c.schemas,
            ..Default::default()
        }),
        _ => None,
    };

    transform
}
