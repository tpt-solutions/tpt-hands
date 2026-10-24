// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"fmt"
	"strconv"
	"time"
)

// timeoutUnits are the grpc-timeout unit letters, from largest to smallest.
var timeoutUnits = []struct {
	letter byte
	unit   time.Duration
}{
	{'H', time.Hour},
	{'M', time.Minute},
	{'S', time.Second},
	{'m', time.Millisecond},
	{'u', time.Microsecond},
	{'n', time.Nanosecond},
}

// encodeTimeout formats d as a grpc-timeout value: at most 8 digits and one
// unit letter. It picks the finest unit that fits in 8 digits. The value is
// rounded down, so a deadline is never extended.
func encodeTimeout(d time.Duration) string {
	if d < 0 {
		d = 0
	}
	for i := len(timeoutUnits) - 1; i >= 0; i-- {
		u := timeoutUnits[i]
		if v := d / u.unit; v < 100_000_000 {
			return strconv.FormatInt(int64(v), 10) + string(u.letter)
		}
	}
	return "99999999H"
}

// parseTimeout reads a grpc-timeout value.
func parseTimeout(s string) (time.Duration, error) {
	if len(s) < 2 || len(s) > 9 {
		return 0, fmt.Errorf("bad grpc-timeout %q", s)
	}
	n, err := strconv.ParseUint(s[:len(s)-1], 10, 64)
	if err != nil {
		return 0, fmt.Errorf("bad grpc-timeout %q", s)
	}
	letter := s[len(s)-1]
	for _, u := range timeoutUnits {
		if u.letter == letter {
			return time.Duration(n) * u.unit, nil
		}
	}
	return 0, fmt.Errorf("bad grpc-timeout unit in %q", s)
}
