# ------------------------------------------
# SPDX-License-Identifier: MIT OR Apache-2.0
# -------------------------------- 𝒒𝒑𝒓𝒐𝒋 --

qproj := "qproj-scripts"
NIXGL := env("NIXGL", "nixVulkanNvidia")

_default:
    just --list

# Build the workspace.
[working-directory: '.']
build *args:
    {{ qproj }} build {{ args }}

# Run the application.
[working-directory: '.']
play *args:
    nix run --impure github:nix-community/nixGL#{{ NIXGL }} -- \
        {{ qproj }} play {{ args }}

# Lint with Clippy.
[working-directory: '.']
check *args:
    cargo clippy {{ args }}

# Run clippy.
[working-directory: '.']
clippy *args:
    cargo clippy {{ args }}

# Check dependencies with cargo-deny.
[working-directory: '.']
deny:
    {{ qproj }} deny

# Run tests via cargo-nextest.
[working-directory: '.']
test *args:
    {{ qproj }} test {{ args }}

# Generate test coverage report.
[working-directory: '.']
coverage *args:
    {{ qproj }} coverage {{ args }}

# Fix all fixable issues.
[working-directory: '.']
fix *args:
    cargo clippy --fix {{ args }}

# Test CI locally with act.
[working-directory: '.']
ci *args:
    {{ qproj }} ci {{ args }}

# List the Bevy spawn sites added or removed between two versions (<old> <new>),
# to audit the persistent types in src/persistent.rs.
[working-directory: '.']
audit-bevy-spawns *args:
    python3 scripts/audit_bevy_spawns.py {{ args }}
