# Load configuration code from one selected source

## Why
Configuration sessions need the original module graph and module sources. The interpreter must receive one immutable, source-neutral snapshot instead of learning the physical `Config`, Designer XML, or EDT formats.

## What Changes
- Add `bsl-config` with the neutral `ConfigurationCodeSnapshot` contract.
- Add `bsl-config-db`, which builds that snapshot from an information base through `open-sdbl-db`.
- Add documented `bsl-config-xml` and `bsl-config-edt` crate placeholders without loader APIs or CLI switches.
- Produce one effective source body for every logical module; active extensions may be applied, but base and patch variants never escape the loader.
- Preserve common-module visibility: a non-global common module is addressed by its module name, while exports of a global common module enter the bare BSL namespace.
- Keep the code source independent from the metadata and data source, so a future XML or EDT code snapshot can run against matching metadata and data acquired from an information base.

## Impact
New crates live above the existing interpreter. `open-bsl`, `bsl-rt`, and `bsl-vm` do not depend on `open-sdbl` or a database driver. Database loading requires the upstream `open-sdbl-db` acquisition API recorded in `docs/plans/open-sdbl-configuration-acquisition-request.md`; this repository does not duplicate or patch that provider implementation.
