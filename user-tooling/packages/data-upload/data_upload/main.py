# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

import argparse
import json
import logging
import os
import re
import socket
import urllib.parse
from datetime import datetime, timezone
from functools import reduce
from typing import Any

import validators
from deepmerge import DEFAULT_TYPE_SPECIFIC_MERGE_STRATEGIES, Merger  # type: ignore
from opensearchpy import OpenSearch, helpers  # type: ignore

logger = logging.getLogger(__name__)

error_on_override: bool = False


def custom_override(_config: Merger, path: list, _prev: dict, final: dict) -> dict:
    """If a leaf item is overridden, we want to see at least a warning as this indicates
    unwanted behaviour. If error_on_override is set, we want an exception."""
    if error_on_override:
        logger.error("Overriding %s while merging two dicts.", path)
        logger.error("Aborting the program because --error-on-override is set.")
        raise AttributeError
    logger.warning(
        "Overriding %s while merging two dicts. This may indicate incorrect usage.",
        path,
    )
    return final


custom_merger = Merger(
    DEFAULT_TYPE_SPECIFIC_MERGE_STRATEGIES, [custom_override], [custom_override]
)


def deepmerge_dicts(dicts: list[dict]) -> dict:
    """Deeply merges a list of dicts."""
    if len(dicts) == 0:
        return {}

    return reduce(custom_merger.merge, dicts, {})


def parse_args() -> Any:
    """Command line parsing."""
    parser = argparse.ArgumentParser(
        description="""Assembles JSON documents from JSON files and pushes these
        documents to OpenSearch."""
    )
    parser.add_argument(
        "--metadata",
        required=False,
        help="""A directory containing JSON files or a single JSON file. The JSON from
        all these files will appear in all JSON documents.""",
        metavar="FILEs",
        action="store",
        type=str,
    )
    parser.add_argument(
        "--results",
        required=True,
        help="""A directory containing JSON files or a single JSON file. For each JSON
        file this tool will create one JSON document (that contains all metadata from
        the --metadata folder or file).""",
        metavar="FILEs",
        action="store",
        type=str,
    )
    parser.add_argument(
        "--url",
        required=False,
        help="""URL to upload the generated JSON documents. The URL must be an
        OpenSearch compatible API for document upload.""",
    )
    parser.add_argument(
        "--token",
        required=False,
        help="""JSON Web Token to authenticate data upload. By default the environment
        variable DATA_UPLOAD_TOKEN will be used for Bearer authentication.""",
    )
    parser.add_argument(
        "--error-on-override",
        required=False,
        action="store_true",
        help="""Instructs this tool to bail out if a leaf item of a JSON document is
        overridden while merging two JSON documents.""",
    )
    return parser.parse_args()


def json_from_file(path: str) -> dict:
    """Reads a JSON file and returns the content as a dict."""

    if not os.path.isfile(path):
        logger.warning("%s is not a file.", path)
        return {}

    try:
        with open(path, encoding="UTF-8") as json_file:
            return json.load(json_file)
    except json.JSONDecodeError as e:
        logger.warning("%s does not contain valid JSON. Error: %s", path, e.msg)
        return {}


def json_from_directory(path: str) -> list[dict]:
    """Parses all JSON files in the given directories and returns the contents as a
    list of dicts."""

    if not os.path.isdir(path):
        logger.warning("%s is not a directory", path)
        return [{}]

    dir_content = [os.path.join(path, f) for f in os.listdir(path)]
    files = [f for f in dir_content if os.path.isfile(f) and f.endswith(".json")]
    return list(map(json_from_file, files))


def collect_ci_data() -> dict:
    """Collects Gitlab information from environment variables."""
    return {
        "ci_branch": os.environ.get("CI_COMMIT_REF_SLUG", "local-branch"),
        "ci_commit": os.environ.get(
            "CI_COMMIT_SHA", "0000000000000000000000000000000000000000"
        ),
        "ci_user": os.environ.get(
            "GITLAB_USER_NAME", os.environ.get("USER", "local-user")
        ),
        "ci_job_name": os.environ.get("CI_JOB_NAME", "local-job"),
        "ci_job_url": os.environ.get("CI_JOB_URL", "local-job"),
        "ci_project_name": os.environ.get("CI_PROJECT_NAME", "local-project"),
        "ci_project_namespace": os.environ.get(
            "CI_PROJECT_NAMESPACE", "local-project-namespace"
        ),
        "ci_project_path": os.environ.get("CI_PROJECT_PATH", "local-project"),
        "ci_project_url": os.environ.get("CI_PROJECT_URL", "local-project"),
        "ci_pipeline_url": os.environ.get("CI_PIPELINE_URL", "local-job"),
    }


def to_kebab_case(name: str) -> str:
    """Converts a given string to kebab case."""
    # E.g. 'HTTPResponseCodeXYZ' -> 'HTTP-ResponseCodeXYZ'
    name = re.sub("(.)([A-Z][a-z]+)", r"\1-\2", name)
    # Replace underscores with dashes in case we get a snake_case styled word
    name = name.replace("_", "-")
    # Replace slashes with dashes. They are not allowed in index store paths.
    name = name.replace("/", "-")
    # Remove multiple consecutive dashes
    name = re.sub("--([A-Z])", r"-\1", name)
    # E.g. 'HTTP-ResponseCodeXYZ' -> 'HTTP-Response-Code-XYZ'
    name = re.sub("([a-z0-9])([A-Z])", r"\1-\2", name)
    # Remove spaces
    name = name.replace(" ", "")
    return name.lower()


def get_index_for_document(document: dict) -> str:
    """Get the store path index for a given document.

    If the script is called from a Gitlab CI Job, this function will use the Gitlab
    environment variables to build the index:

    {ci_project_path}-{benchmark_name}
    E.g. engineering-infrastructure-analytics-vehicle-performance

    If the environment variables are not available, the index will be build like that:
    {username}-{benchmark_name}
    E.g. bob-vehicle-performance

    If no USER environment variables is set the name "local-user" will be used.
    """

    index_prefix: str = os.environ.get(
        "CI_PROJECT_PATH", os.environ.get("USER", "local-user")
    )

    if benchmark_name := document.get("name"):
        index: str = f"{index_prefix}-{benchmark_name}"
        return to_kebab_case(index)

    logger.error("Got a document without a name:\n%s", json.dumps(document, indent=2))
    raise KeyError()


def upload_to_datastore(upload_url: str, token: str, documents: list[dict]) -> None:
    """Use the OpenSearch Bulk API to upload the given data."""

    # The port is a mandatory part of the URL specification starting with elastic 8.0.
    # Transparently add it if it is missing.
    url = urllib.parse.urlparse(upload_url)
    if not url.port:
        port: str = str(socket.getservbyname(url.scheme))
        url = url._replace(netloc=f"{url.netloc}:{port}")
        upload_url = url.geturl()

    es = OpenSearch([upload_url])
    actions: list[dict] = [
        {
            "_op_type": "create",
            "_index": get_index_for_document(document),
            "_source": document,
        }
        for document in documents
    ]

    _, errors = helpers.bulk(es, actions, headers={"authorization": f"Bearer {token}"})

    num_errors: int = 0
    if isinstance(errors, dict):
        num_errors = len(errors)

    if num_errors > 0:
        logger.warning(
            "Got %s warnings while uploading documents to '%s': %s",
            num_errors,
            upload_url,
            errors,
        )


def upload_data() -> None:
    args = parse_args()

    if not os.path.exists(args.results):
        raise ValueError(f"{args.results}: The file or folder does not exist.")

    if args.metadata and not os.path.exists(args.metadata):
        raise ValueError(f"{args.metadata}: The file or folder does not exist.")

    if args.url and not validators.url(args.url):
        raise ValueError(f"Provided upload url is not a valid url (${args.url})")

    if args.error_on_override:
        global error_on_override
        error_on_override = True

    metadata_list = [{}] if not args.metadata else json_from_directory(args.metadata)
    metadata: dict = deepmerge_dicts(metadata_list)

    ci_data: dict = collect_ci_data()
    # Merge ci_data into metadata. We put ci_data first, that way a user can overwrite
    # it for local invocations of this tool by providing a JSON file that contains
    # `ci_xxx` keys.
    metadata = deepmerge_dicts([ci_data, metadata])

    result_list = json_from_directory(args.results)

    documents = [deepmerge_dicts([metadata, result]) for result in result_list]

    # Before we upload the documents, we make sure that every document has a timestamp
    for document in documents:
        if not document.get("timestamp"):
            logger.warning(
                "Found a document without a timestamp. Adding current timestamp."
            )
            document["timestamp"] = datetime.now(timezone.utc).isoformat(
                timespec="milliseconds"
            )

    if args.url:
        token = (
            os.environ.get("DATA_UPLOAD_TOKEN", "NO_TOKEN_SUPPLIED")
            if args.token is None
            else args.token
        )
        upload_to_datastore(args.url, token, documents)
    else:
        print(json.dumps(documents, indent=2))
