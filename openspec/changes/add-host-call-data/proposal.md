# Carry session host data through component calls

## Why
Platform data adapters need supported host extension points without source rewriting or database dependencies in the interpreter.

## What Changes
Carry session host data through component calls. This change defines the host contract agreed during plan review.

## Impact
Affected areas: runtime, resolver where applicable, VM, facade and facade-only consumer tests. No new external dependencies or bytecode format changes. Rust API may evolve; existing BSL behavior remains unchanged unless a host explicitly registers a Metadata replacement.
