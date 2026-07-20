# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ pkgs, benchmarkDataCollector }:
let
  py = pkgs.python3Packages;

  pip-licenses = py.buildPythonApplication rec {
    pname = "pip-licenses";
    version = "5.5.0";
    pyproject = true;

    src = pkgs.fetchPypi {
      pname = "pip_licenses";
      inherit version;
      hash = "sha256-JHPnr9AqDCFGB1j3D9K7OzwIDFFQcT3TO6qUk9wVY6U=";
    };

    build-system = [
      py.setuptools
      py.setuptools-scm
    ];

    dependencies = [
      py.prettytable
    ];
  };

  generallyAllowedLicenses = [
    "MIT"
    "MIT License"
    "Apache-2.0"
    "Apache 2.0"
    "Apache Software License"
    "BSD-2-Clause"
    "BSD-3-Clause"
    "BSD License"
    "Apache-2.0 OR BSD-2-Clause"
    "PSF-2.0"
  ];
  generallyAllowedLicensesString = builtins.concatStringsSep ";" generallyAllowedLicenses;

  # These licenses are not generally allowed, but the packages were manually verified.
  # See NOTICE file in repository root for details.
  licenseExceptions = [
    {
      package = "pathspec";
      license = "Mozilla Public License 2.0 (MPL 2.0)";
    }
    {
      package = "certifi";
      license = "Mozilla Public License 2.0 (MPL 2.0)";
    }
    {
      package = "tqdm";
      license = "MPL-2.0 AND MIT";
    }
  ];
  licenseExceptionsPackagesString = pkgs.lib.strings.concatMapStringsSep " " (p: p.package) licenseExceptions;
  licenseExceptionsLicensesString = pkgs.lib.strings.concatMapStringsSep ";" (p: p.license) licenseExceptions;
in
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
  nativeCheckInputs = with pkgs.python3Packages; [ black mypy pkgs.ruff pytest responses types-tqdm pip-licenses ];
  checkPhase = ''
    echo "## run mypy"
    mypy hwaas_driver
    echo "## run ruff"
    ruff check .
    echo "## run black"
    black --check --diff .
    echo "## run pytest"
    pytest
    echo "## check generally allowed Python dependency licenses"
    pip-licenses \
      --from=mixed \
      --format=plain \
      --with-urls \
      --ignore-packages ${licenseExceptionsPackagesString} \
      --allow-only='${generallyAllowedLicensesString}'
    echo "## check approved licensing exceptions"
    pip-licenses \
      --from=mixed \
      --format=plain \
      --with-urls \
      --packages ${licenseExceptionsPackagesString} \
      --allow-only='${licenseExceptionsLicensesString}'
  '';
}
