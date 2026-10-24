# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions

"""Shared method names and message types for the grpc-core interop checks.

Messages are google.protobuf wrapper types, which the protobuf runtime ships,
so no code generation is needed. The Go side uses the same wrapper types.
"""

from google.protobuf import wrappers_pb2

SERVICE = "test.Echo"
STRING = wrappers_pb2.StringValue
INT64 = wrappers_pb2.Int64Value
