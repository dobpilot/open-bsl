# Change: Bind flat BSL array query parameters

## Why
Original demo reaches a reference IN parameter in ValuesOfObjectsAttributes. The adapter rejects Array before SQL; a later metadata-index error masks this first failure.

## What Changes
Convert flat BSL arrays through the existing ParameterValue.List and scalar parameter codec at query execution, including empty arrays. Preserve the existing live query parameter values.

## Impact
Database adapter parameter conversion and focused tests. No parser, compiler, VM, SDK API or dependency changes.
