# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, benchmarkDataCollector }:
pkgs.python3Packages.buildPythonPackage {
  name = "hwaas-driver";
  src = ./.;

  nativeBuildInputs = with pkgs.python3Packages; [
    setuptools
  ];

  propagatedBuildInputs = with pkgs.python3Packages; [
    benchmarkDataCollector
    requests
    types-requests
    requests-toolbelt
    tqdm
  ];

  pyproject = true;

  doCheck = true;
  nativeCheckInputs = with pkgs.python3Packages; [ black mypy pkgs.ruff pytest responses types-tqdm ];
  checkPhase = ''
    echo "## run mypy"
    mypy hwaas_driver
    echo "## run ruff"
    ruff check .
    echo "## run black"
    black --check --diff .
    echo "## run pytest"
    pytest
  '';
}
