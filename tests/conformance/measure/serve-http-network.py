#!/usr/bin/env python3
"""Локальные IPv4/IPv6 HTTP-точки для measure-http-network.bsl."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from socket import AF_INET6
from threading import Event, Thread
from time import sleep


class TargetHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/timeout-before-headers":
            sleep(3)
        elif self.path == "/timeout-cumulative":
            sleep(0.6)
        self.send_response(200)
        if self.path == "/duplicate-headers":
            self.send_header("X-Duplicate", "first")
            self.send_header("X-Duplicate", "second")
        if self.path == "/response-body":
            body = b"\xef\xbb\xbf\xd0\x90\xd0\xb1"
            self.send_header("Content-Type", "text/plain; charset=utf-8")
        else:
            body = b"ok" if self.path in {"/timeout-between-body", "/timeout-cumulative"} else b""
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        if not body:
            return
        try:
            self.wfile.write(body[:1])
            self.wfile.flush()
            sleep(3 if self.path == "/timeout-between-body" else 0.6)
            self.wfile.write(body[1:])
        except (BrokenPipeError, ConnectionResetError):
            pass

    def log_message(self, _format, *_arguments):
        pass


class ProxyHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(299)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def log_message(self, _format, *_arguments):
        pass


class IPv6HTTPServer(ThreadingHTTPServer):
    address_family = AF_INET6


def serve(server_type, address, port, handler, ready):
    try:
        server = server_type((address, port), handler)
    except OSError as error:
        print(f"{address}:{port}: {error}", flush=True)
        ready.set()
        return
    ready.set()
    server.serve_forever()


def main():
    ready = [Event(), Event(), Event()]
    endpoints = [
        (ThreadingHTTPServer, "127.0.0.1", 80, TargetHandler),
        (ThreadingHTTPServer, "127.0.0.1", 18080, TargetHandler),
        (ThreadingHTTPServer, "127.0.0.1", 18081, ProxyHandler),
        (ThreadingHTTPServer, "127.0.0.2", 18083, TargetHandler),
        (ThreadingHTTPServer, "0.0.0.0", 18084, TargetHandler),
        (IPv6HTTPServer, "::1", 18082, TargetHandler),
    ]
    ready.extend([Event(), Event(), Event()])
    for (server_type, address, port, handler), started in zip(endpoints, ready):
        Thread(
            target=serve,
            args=(server_type, address, port, handler, started),
            daemon=True,
        ).start()
    for started in ready:
        started.wait()
    Event().wait()


if __name__ == "__main__":
    main()
