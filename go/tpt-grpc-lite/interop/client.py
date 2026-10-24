# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions

"""grpc-core client that runs the interop checks against a Go server.

Usage: python client.py HOST:PORT. Exits 0 when every check passes, and 1 on
the first failure, printing what went wrong.
"""

import sys

import grpc

from common import INT64, SERVICE, STRING


def method(name):
    return "/%s/%s" % (SERVICE, name)


def check(name, got, want):
    if got != want:
        raise SystemExit("FAIL %s: got %r, want %r" % (name, got, want))
    print("ok", name, flush=True)


def main():
    ch = grpc.insecure_channel(sys.argv[1])
    grpc.channel_ready_future(ch).result(timeout=10)

    unary = ch.unary_unary(method("Unary"), request_serializer=lambda m: m.SerializeToString(),
                           response_deserializer=STRING.FromString)
    check("unary", unary(STRING(value="hi")).value, "echo:hi")

    fail = ch.unary_unary(method("Fail"), request_serializer=lambda m: m.SerializeToString(),
                          response_deserializer=STRING.FromString)
    try:
        fail(STRING(value="héllo 100%"))
        raise SystemExit("FAIL error status: no error")
    except grpc.RpcError as e:
        check("error code", e.code(), grpc.StatusCode.NOT_FOUND)
        check("error message", e.details(), "no such thing: héllo 100%")

    missing = ch.unary_unary("/%s/Nope" % SERVICE, request_serializer=lambda m: m.SerializeToString(),
                             response_deserializer=STRING.FromString)
    try:
        missing(STRING(value="x"))
        raise SystemExit("FAIL unknown method: no error")
    except grpc.RpcError as e:
        check("unknown method", e.code(), grpc.StatusCode.UNIMPLEMENTED)

    meta = ch.unary_unary(method("Meta"), request_serializer=lambda m: m.SerializeToString(),
                          response_deserializer=STRING.FromString)
    got = meta(STRING(value="x"), metadata=[("x-user", "ann"), ("x-blob-bin", b"\x01\x02")])
    check("metadata and binary metadata", got.value, "ann|\x01\x02")

    slow = ch.unary_unary(method("Slow"), request_serializer=lambda m: m.SerializeToString(),
                          response_deserializer=STRING.FromString)
    try:
        slow(STRING(value="x"), timeout=0.2)
        raise SystemExit("FAIL deadline: no error")
    except grpc.RpcError as e:
        check("deadline exceeded", e.code(), grpc.StatusCode.DEADLINE_EXCEEDED)

    count = ch.unary_stream(method("Count"), request_serializer=lambda m: m.SerializeToString(),
                            response_deserializer=INT64.FromString)
    check("server streaming", [m.value for m in count(INT64(value=5))], [1, 2, 3, 4, 5])

    total = ch.stream_unary(method("Sum"), request_serializer=lambda m: m.SerializeToString(),
                            response_deserializer=INT64.FromString)
    check("client streaming", total(iter([INT64(value=i) for i in range(1, 11)])).value, 55)

    upper = ch.stream_stream(method("Upper"), request_serializer=lambda m: m.SerializeToString(),
                             response_deserializer=STRING.FromString)
    words = ["alpha", "beta", "gamma"]
    check("bidi streaming", [m.value for m in upper(iter([STRING(value=w) for w in words]))],
          [w.upper() for w in words])

    print("all interop checks passed")


if __name__ == "__main__":
    main()
