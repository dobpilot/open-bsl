# Support reached SHA512 hashing

## Why
Original demo startup reaches DataHashing(HashFunction.SHA512), which currently rejects a valid hash enumeration value.

## What Changes
Compute SHA512 through the existing sha2 dependency, preserving incremental input and non-consuming hash snapshots.

## Impact
Only bsl-crypto algorithm dispatch changes. No dependencies, resolver changes or public declarations are added.
