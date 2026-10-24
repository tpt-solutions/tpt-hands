// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"fmt"

	"google.golang.org/protobuf/proto"
)

// Codec turns messages into bytes and back. The default is ProtoCodec.
type Codec interface {
	Marshal(v any) ([]byte, error)
	Unmarshal(data []byte, v any) error
}

// ProtoCodec encodes protobuf messages (proto.Message values).
type ProtoCodec struct{}

// Marshal encodes v, which must be a proto.Message.
func (ProtoCodec) Marshal(v any) ([]byte, error) {
	m, ok := v.(proto.Message)
	if !ok {
		return nil, fmt.Errorf("ProtoCodec: %T is not a proto.Message", v)
	}
	return proto.Marshal(m)
}

// Unmarshal decodes data into v, which must be a proto.Message.
func (ProtoCodec) Unmarshal(data []byte, v any) error {
	m, ok := v.(proto.Message)
	if !ok {
		return fmt.Errorf("ProtoCodec: %T is not a proto.Message", v)
	}
	return proto.Unmarshal(data, m)
}
