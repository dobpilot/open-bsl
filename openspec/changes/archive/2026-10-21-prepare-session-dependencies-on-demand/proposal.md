# Prepare session dependencies on demand

## Why

Configured startup currently roots every server common module. BUH therefore
spends 174.74 seconds selecting and analyzing bodies and 98.51 seconds verifying
the resulting catalog even with two preparation workers. Resource acquisition
and ORM construction also eagerly decode descriptions outside the startup graph.
The required unit of preparation is the session module and its dependencies,
not the entire configuration.

## What Changes

- Start preparation from the session module and the supplied entry sources.
- Keep a declaration index for name resolution without preparing every body.
- Load, resolve, compile and verify only reached modules and their dependencies.
- Materialize dynamic dependencies at first access and cache immutable results
  in the factory; keep mutable module instances and parameters session-local.
- Keep metadata identity and collection discovery separate from detailed object
  materialization. Read and decode details only when required.
- Extend the database adapter only where its existing acquisition API cannot
  supply indexed and individually requested resources. Keep provider SQL there.
- Preserve the standalone CLI and the existing explicitly whole-catalog APIs.

## Impact

This affects `bsl-config`, `bsl-config-db`, `bsl-orm`, `bsl-session`, the
`open-bsl` facade, compiler integration and neutral VM catalog services.
It changes when unused source or metadata errors are reported. It does not
introduce a second BSL resolver or change BSL name precedence.

The user explicitly selected first-access loading for runtime-computed names.
Simply removing common modules from the current roots would break that
requirement and is not an acceptable implementation.
