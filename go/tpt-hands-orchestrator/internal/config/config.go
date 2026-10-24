// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

// Package config reads the orchestrator's settings from environment variables.
package config

import (
	"fmt"
	"log/slog"
	"os"
	"strings"
)

// Config holds the orchestrator's runtime settings.
type Config struct {
	// ListenAddr is the address the health and metrics endpoint binds to.
	ListenAddr string
	// LogLevel is one of debug, info, warn or error.
	LogLevel slog.Level
}

// Environment variable names.
const (
	EnvListenAddr = "TPT_HANDS_LISTEN_ADDR"
	EnvLogLevel   = "TPT_HANDS_LOG_LEVEL"
)

// Defaults used when a variable is unset.
const (
	DefaultListenAddr = "127.0.0.1:8080"
	DefaultLogLevel   = "info"
)

// FromEnv builds a Config from the environment, falling back to defaults.
// It returns an error for a value it cannot use, so a typo fails at startup.
func FromEnv(lookup func(string) (string, bool)) (Config, error) {
	cfg := Config{ListenAddr: DefaultListenAddr}

	if v, ok := lookup(EnvListenAddr); ok && strings.TrimSpace(v) != "" {
		cfg.ListenAddr = strings.TrimSpace(v)
	}

	level := DefaultLogLevel
	if v, ok := lookup(EnvLogLevel); ok && strings.TrimSpace(v) != "" {
		level = strings.ToLower(strings.TrimSpace(v))
	}
	lvl, err := parseLevel(level)
	if err != nil {
		return Config{}, err
	}
	cfg.LogLevel = lvl
	return cfg, nil
}

// FromOS is FromEnv reading the process environment.
func FromOS() (Config, error) {
	return FromEnv(os.LookupEnv)
}

func parseLevel(s string) (slog.Level, error) {
	switch s {
	case "debug":
		return slog.LevelDebug, nil
	case "info":
		return slog.LevelInfo, nil
	case "warn":
		return slog.LevelWarn, nil
	case "error":
		return slog.LevelError, nil
	default:
		return 0, fmt.Errorf("config: %s: unknown log level %q", EnvLogLevel, s)
	}
}
