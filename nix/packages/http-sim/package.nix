# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ python3Packages }:
python3Packages.buildPythonApplication {
  pname = "http-sim";
  version = "1.0";

  format = "pyproject";
  build-system = [ python3Packages.setuptools ];

  propagatedBuildInputs = with python3Packages; [
    fastapi
    requests
    pydantic
    uvicorn
  ];
  src = ./.;

  doCheck = false;
}
