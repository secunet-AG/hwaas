#!/usr/bin/env python

# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

from fastapi import FastAPI, Response
import uvicorn
from pydantic import BaseModel

app = FastAPI()

@app.get("/{status_code}")
def root(status_code: int, response: Response):
    response.status_code = status_code

class ResponseConfig(BaseModel):
    media_type: str
    status_code: int | None = 200
    headers: dict[str,str]
    content: str | None = "empty"

@app.post("/craft")
def root(conf: ResponseConfig, response: Response):
    return Response(
        media_type=conf.media_type,
        status_code=conf.status_code,
        headers=conf.headers,
        content=conf.content,
    )

@app.delete("/")
def root(response: Response):
    response.status_code = 200

if __name__ == '__main__':
    config = uvicorn.Config(app, port=5042, log_level="info")
    server = uvicorn.Server(config)
    server.run()
