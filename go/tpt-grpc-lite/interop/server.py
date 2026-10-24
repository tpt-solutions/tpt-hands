# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions

"""grpc-core server that implements the test.Echo fixture, for interop tests.

Usage: python server.py PORT. Serves until killed. Behaviour matches the Go
fixture in grpclite_test.go.
"""

import sys
import time
from concurrent import futures

import grpc

from common import INT64, SERVICE, STRING


def echo(req, ctx):
    return STRING(value="echo:" + req.value)


def fail(req, ctx):
    ctx.abort(grpc.StatusCode.NOT_FOUND, "no such thing: " + req.value)


def slow(req, ctx):
    # Wait for the client's deadline. grpc-core cancels the context when it
    # expires, so is_active() goes false and the handler can stop.
    while ctx.is_active():
        time.sleep(0.01)
    return STRING(value="late")


def meta(req, ctx):
    md = dict(ctx.invocation_metadata())
    # Keys ending in -bin arrive as bytes.
    blob = md.get("x-blob-bin", b"")
    return STRING(value=md.get("x-user", "") + "|" + blob.decode("latin-1"))


def count(req, ctx):
    for i in range(1, req.value + 1):
        yield INT64(value=i)


def total(req_iter, ctx):
    return INT64(value=sum(m.value for m in req_iter))


def upper(req_iter, ctx):
    for m in req_iter:
        yield STRING(value=m.value.upper())


def ser(m):
    return m.SerializeToString()


def handler():
    return grpc.method_handlers_generic_handler(SERVICE, {
        "Unary": grpc.unary_unary_rpc_method_handler(
            echo, request_deserializer=STRING.FromString, response_serializer=ser),
        "Fail": grpc.unary_unary_rpc_method_handler(
            fail, request_deserializer=STRING.FromString, response_serializer=ser),
        "Slow": grpc.unary_unary_rpc_method_handler(
            slow, request_deserializer=STRING.FromString, response_serializer=ser),
        "Meta": grpc.unary_unary_rpc_method_handler(
            meta, request_deserializer=STRING.FromString, response_serializer=ser),
        "Count": grpc.unary_stream_rpc_method_handler(
            count, request_deserializer=INT64.FromString, response_serializer=ser),
        "Sum": grpc.stream_unary_rpc_method_handler(
            total, request_deserializer=INT64.FromString, response_serializer=ser),
        "Upper": grpc.stream_stream_rpc_method_handler(
            upper, request_deserializer=STRING.FromString, response_serializer=ser),
    })


def main():
    port = int(sys.argv[1])
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=16))
    server.add_generic_rpc_handlers((handler(),))
    server.add_insecure_port("127.0.0.1:%d" % port)
    server.start()
    print("ready", flush=True)
    server.wait_for_termination()


if __name__ == "__main__":
    main()
