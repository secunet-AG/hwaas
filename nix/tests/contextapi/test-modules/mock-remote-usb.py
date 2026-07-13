# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

import json
from urllib import parse
from http import server as http_server
from threading import Thread


class Handler(http_server.BaseHTTPRequestHandler):
    # Mocking PUT /usb here
    # Returns the body from the request as the response, since this is the same
    # as what happens in remote-hands (the name of the types for input and
    # output are different, but the deserialization leads to the same json
    # representation)
    def do_PUT(self):
        # get body to return it in response
        content_length = int(self.headers.get("content-length", 0))
        body = self.rfile.read(content_length).decode("utf-8")

        # get path and print for logging
        parsed_path = parse.urlparse(self.path)
        print(f"HTTP PUT '{parsed_path.path}' with body {body}", flush=True)

        # add headers
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()

        # write response
        self.wfile.write(body.encode("utf-8"))

    # Mocking DELETE /usb here
    # This mock only returns a successful status code like remote-hands would
    def do_DELETE(self):
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"")

    # Mocking GET /usb here
    # This mock does not support any state, therefore simply return empty list
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(b"[]")

    # Mocking POST /usb/*rest here
    def do_POST(self):
        # get body to return it in response
        content_length = int(self.headers.get("content-length", 0))
        body = self.rfile.read(content_length).decode("utf-8")

        # get path and print for logging
        parsed_path = parse.urlparse(self.path)
        print(f"HTTP POST '{parsed_path.path}' with body '{body}'", flush=True)

        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"")


with http_server.HTTPServer(("127.0.0.1", 8081), Handler) as server:
    server.serve_forever()
