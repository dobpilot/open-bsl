# Why

> Status: deferred by explicit user decision on 2026-10-21. Full execution
> without stubs is outside the current session-initialization goal and does
> not block its completion. Preserve implementation, fixtures and unfinished
> tasks; resume this work only on a separate request.

The buh configuration contains a common module whose native module container
has an executable `image` and no BSL `text`. Native XML export preserves it as
`Module.bin`. Treating that module as empty loses its declarations and prevents
the original configuration from executing. The user requires a decompiler.

# What Changes

- Recover executable BSL from supported native module images when source text
  is absent, within the selected configuration code loader.
- Preserve one effective module body and compile recovered BSL through the
  existing open-bsl resolver, compiler and VM.
- Reject unsupported or malformed images explicitly; never replace them with
  an empty module or successful stub.
- Establish supported format rules using controlled native platform artifacts
  and execution receipts.

# Impact

Configuration acquisition and code loading change. No database dependency is
introduced into the runtime, VM or facade, and no new external dependency is
required. XML and EDT reader implementations remain outside this change.
