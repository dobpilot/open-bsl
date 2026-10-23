# Measure startup phases and reuse rooted analysis

## Why
The BUH cold CLI process takes 800.57 seconds, but the completed run has no phase timings. Inspection shows a full export-surface parse, dependency-discovery parsing/resolution, and a second parsing/resolution pass for compilation. The import-storage correction removes retained quadratic data without eliminating this repeated CPU work.

## What Changes
Add opt-in stderr startup timing records controlled by OPEN_BSL_STARTUP_TIMINGS=1. Preserve default output. Reuse selected modules' resolved bodies from dependency discovery for bytecode generation, retaining the existing declaration-surface scan and only one live body AST at a time. Rebuild each final import environment and remap original module IDs to compact selected-catalog IDs.

## Impact
No BSL semantics, public Rust signature, dependency, bytecode format, visibility or session isolation changes. Existing tests and baseline artifacts stay intact. Validate bytecode equivalence and run alternating baseline/candidate BUH processes sequentially with the established RAM/time guards before claiming a speed improvement.
