# Change: Separate map key access from iteration

## Why
Original demo groups references using Map[TypeOf(reference)]. Runtime still treats every non-string map index as an iteration position, despite the dedicated collection iteration operation.

## What Changes
Move map positional pair retrieval to collection_item and use key lookup for all ordinary map brackets. Preserve insertion and loop semantics.

## Impact
Base runtime map indexing/iteration, focused regression and original demo replay. No opcode, format, dependency or resolver changes.
