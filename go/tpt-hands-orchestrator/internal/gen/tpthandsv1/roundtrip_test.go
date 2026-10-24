// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package tpthandsv1

import (
	"testing"

	"google.golang.org/protobuf/proto"
)

func TestPathwayHealthRoundTrip(t *testing.T) {
	in := &PathwayHealth{
		Id:               2,
		State:            PathwayState_PATHWAY_STATE_SEVERED,
		HealingProgress:  0.25,
		ResistanceOhms:   480.5,
		MaxBandwidthMbps: 12,
	}
	buf, err := proto.Marshal(in)
	if err != nil {
		t.Fatal(err)
	}
	var out PathwayHealth
	if err := proto.Unmarshal(buf, &out); err != nil {
		t.Fatal(err)
	}
	if !proto.Equal(in, &out) {
		t.Errorf("round trip changed the message: got %v, want %v", &out, in)
	}
}

func TestMorphogenesisRequestKeepsRequestID(t *testing.T) {
	in := &MorphogenesisRequest{ToolId: "finger-0", RequestId: "req-42"}
	buf, err := proto.Marshal(in)
	if err != nil {
		t.Fatal(err)
	}
	var out MorphogenesisRequest
	if err := proto.Unmarshal(buf, &out); err != nil {
		t.Fatal(err)
	}
	if out.GetRequestId() != "req-42" || out.GetToolId() != "finger-0" {
		t.Errorf("got tool %q request %q", out.GetToolId(), out.GetRequestId())
	}
}
