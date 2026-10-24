// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package config

import (
	"log/slog"
	"maps"
	"testing"
)

func lookupFrom(m map[string]string) func(string) (string, bool) {
	return func(k string) (string, bool) {
		v, ok := m[k]
		return v, ok
	}
}

func TestDefaults(t *testing.T) {
	cfg, err := FromEnv(lookupFrom(nil))
	if err != nil {
		t.Fatal(err)
	}
	if cfg.ListenAddr != DefaultListenAddr {
		t.Errorf("ListenAddr = %q, want %q", cfg.ListenAddr, DefaultListenAddr)
	}
	if cfg.LogLevel != slog.LevelInfo {
		t.Errorf("LogLevel = %v, want info", cfg.LogLevel)
	}
}

func TestOverrides(t *testing.T) {
	env := map[string]string{
		EnvListenAddr: " 0.0.0.0:9000 ",
		EnvLogLevel:   "DEBUG",
	}
	cfg, err := FromEnv(lookupFrom(env))
	if err != nil {
		t.Fatal(err)
	}
	if cfg.ListenAddr != "0.0.0.0:9000" {
		t.Errorf("ListenAddr = %q", cfg.ListenAddr)
	}
	if cfg.LogLevel != slog.LevelDebug {
		t.Errorf("LogLevel = %v, want debug", cfg.LogLevel)
	}
}

func TestBadLogLevelFailsAtStartup(t *testing.T) {
	env := maps.Clone(map[string]string{EnvLogLevel: "verbose"})
	if _, err := FromEnv(lookupFrom(env)); err == nil {
		t.Fatal("want error for unknown log level")
	}
}
