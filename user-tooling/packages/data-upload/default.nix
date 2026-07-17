# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs
}:
pkgs.python3Packages.buildPythonPackage {
  name = "hwaas-data-upload";
  src = ./.;

  nativeBuildInputs = with pkgs.python3Packages; [
    setuptools
  ];

  propagatedBuildInputs = with pkgs.python3Packages; [
    deepmerge
    validators
    opensearch-py
  ];

  pyproject = true;

  doCheck = true;
  nativeCheckInputs = with pkgs.python3Packages; [ black mypy pkgs.ruff pytest deepmerge ];
  checkPhase = ''
    echo "## run mypy"
    mypy data_upload
    echo "## run ruff"
    ruff check .
    echo "## run black"
    black --check --diff .
    echo "## run pytest"
    pytest
  '';
}
