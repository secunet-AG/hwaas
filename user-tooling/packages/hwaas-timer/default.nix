# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ buildPythonPackage
, black
, mypy
, pytest
, ruff
, setuptools
}:
buildPythonPackage {
  name = "hwaas-timer";
  src = ./.;

  nativeBuildInputs = [
    setuptools
  ];

  pyproject = true;

  doCheck = true;
  nativeCheckInputs = [ black mypy ruff pytest ];
  checkPhase = ''
    echo "## run mypy"
    mypy hwaas_timer
    echo "## run ruff"
    ruff check .
    echo "## run black"
    black --check --diff .
    echo "## run pytest"
    pytest
  '';
}
