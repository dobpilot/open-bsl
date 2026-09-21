# Tasks

- [ ] 1. Obtain the combined configuration acquisition and selectable-user APIs requested in `docs/plans/open-sdbl-configuration-acquisition-request.md`, then pin the `open-sdbl` dependencies reproducibly.
- [x] 2. Add `bsl-config` and test construction and validation of one immutable logical module graph.
- [ ] 3. Add `bsl-config-db`, decode the database configuration resources, and produce one effective source for every module.
- [ ] 4. Test active-extension application and explicit rejection of an extension form that cannot be applied.
- [x] 5. Add dependency-free documented `bsl-config-xml` and `bsl-config-edt` placeholder crates without public loader functions.
- [x] 6. Verify that the existing `bsl-rt` and `bsl-vm` normal dependency trees are unchanged.
- [ ] 7. Validate this change with `openspec validate load-database-configuration --strict`; archive it only after the database loader and its integration gates pass.
