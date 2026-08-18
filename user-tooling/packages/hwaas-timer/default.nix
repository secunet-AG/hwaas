# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  pkgs,
  buildPythonPackage,
  black,
  mypy,
  pytest,
  ruff,
  setuptools,
}:
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

    dependencies = [ py.prettytable ];
  };

  generallyAllowedLicenses = [
    "MIT"
    "MIT License"
    "Apache-2.0"
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
  ];
  licenseExceptionsPackagesString = pkgs.lib.strings.concatMapStringsSep " " (
    p: p.package
  ) licenseExceptions;
  licenseExceptionsLicensesString = pkgs.lib.strings.concatMapStringsSep ";" (
    p: p.license
  ) licenseExceptions;
in
buildPythonPackage {
  name = "hwaas-timer";
  src = ./.;

  nativeBuildInputs = [ setuptools ];

  pyproject = true;

  doCheck = true;
  nativeCheckInputs = [
    black
    mypy
    ruff
    pytest
    pip-licenses
  ];
  checkPhase = ''
    echo "## run mypy"
    mypy hwaas_timer
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
