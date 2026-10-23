# Tasks

## 1. Contract and native evidence
- [x] Inspect existing source, factory, ORM/artifact consumers, placeholder crates and active OpenSpec changes.
- [x] Define the mixed-source ownership, CLI, identity, extension and cache contracts before implementation.
- [ ] Retain matched Configurator XML, native EDT and DB-identity fixtures with producer versions and provenance; include forms, nested attributes and at least text/spreadsheet templates.
- [ ] Add failing tests demonstrating edited local BSL, form metadata and template content taking precedence over DB source payloads.
- [ ] Record every native format/observable semantic mapping against fixture evidence; add required unmeasured markers/registry/measure lines for any unresolved platform behavior implemented later.
- [x] Validate this change with openspec validate load-file-configuration-with-database-orm --strict before implementation.

## 2. Neutral source/artifact contracts
- [x] Add minimal source-neutral description/artifact interfaces beside ConfigurationSource, with explicit owner/kind/identity, absence/empty/error states and origin diagnostics.
- [x] Expose new required host APIs through open-bsl and test a facade-only consumer; keep existing source providers and constructors working.
- [x] Reuse or lift the existing XML parser and existing payload decoders without a duplicate parser or new external dependency.

## 3. Native directory readers
- [x] Implement bsl-config-xml native discovery, identities, module declarations/execution flags and on-demand original BSL reads with fixture tests.
- [x] Implement bsl-config-edt against retained native EDT fixtures, returning the same neutral contract without an IDE/conversion dependency.
- [x] Implement source configuration/object descriptions, form attribute/type/binding/command/handler metadata and module linkage for both readers.
- [x] Implement complete common/owner template acquisition, kind preservation and supported existing-decoder materialization; test explicit unsupported-kind errors.
- [x] Test Unicode names, supported encodings, missing declared resources, malformed descriptions and required binary-only modules without successful placeholder synthesis.
- [x] Implement explicit same-format extension-directory binding and the existing ordered effective-module composition; test missing, mismatched, inactive and unsupported extensions without DB fallback.

## 4. Database composition and runtime consumers
- [x] Split DB ORM/session acquisition from DB source-body acquisition in file mode; use counting-provider tests to distinguish required DB descriptors from prohibited source-body reads.
- [x] Verify native root identity independently of DB storage namespace and bind persistent owners by identity/kind; accept edited compatible local code.
- [x] Test missing persistent owners/fields and incompatible persistent types while accepting legitimate nonpersistent form attributes.
- [x] Attach file descriptions/artifacts to existing metadata and template consumers without another global metadata object or SQL mappings from files.
- [x] Compose the selected bundle through the existing source-backed SessionFactory; retain resolver priority, module instance behavior and deferred compilation policies.
- [ ] Test DB principals/rights/RLS/separation and per-session connections against local role-description changes; no live database writes.
- [x] Test lazy body/artifact acquisition, dynamic lookup, shared immutable caches, detected source edits and isolated mutable state across repeated Sessions.

## 5. CLI and documentation
- [x] Add and document --config-xml/--config-edt and explicit --config-extension bindings through existing CLI option tables.
- [x] Test both argument forms, selector conflicts/duplicates/empty values, no-DB rejection before connect and script-argument boundaries.
- [ ] Preserve existing database-only/default standalone behavior and assertions; make necessary mechanical existing-test adaptations directly under the user's updated instruction.
- [x] Document supported native format versions, source ownership, compatibility errors, extension input and factory-lifetime directory stability in Russian.
- [ ] Document public Rust APIs in Russian with failure conditions; regenerate the BSL reference only if public BSL declarations actually change.

## 6. Verification and delivery
- [x] Run fixture-based XML/EDT parity and facade-only consumer tests for modules, source descriptions, forms and templates.
- [ ] Run read-only acceptance with demo, buh and unf against matched file artifacts, with heavy bsl-cli runs strictly sequential under the existing lock/memory guard.
- [ ] Verify no DB code fallback, no unused form/template payload acquisition and no full-tree/Config rehash per Session using counting tests.
- [ ] Record startup phases and peak RAM for the selected file modes; use serial alternating A/B against an immutable baseline binary before claiming any performance gain.
- [ ] Run cargo fmt --all -- --check.
- [ ] Run cargo clippy --workspace --all-targets -- -D warnings.
- [ ] Run cargo test --workspace, including optimizer conformance, API-reference and open-question registry checks.
- [ ] Run RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps and build open-bsl-consumer.
- [x] Compare cargo tree -p bsl-rt -e normal and cargo tree -p bsl-vm -e normal with their pre-change baselines; inspect file-adapter dependency boundaries.
- [ ] Revalidate strictly, review implementation/scope, merge deltas and archive this change only after the implementation and required checks are complete.

## 7. Full-directory acceptance failures
- [x] Project the measured UUID declaration to the existing native UUID type and test the retained Config/XML/EDT pair.
- [x] Register measured calculation-chart reference descriptors and test complete generic-family expansion and both concrete aliases.
- [x] Apply the existing protected-image policy to selected native file bytes, including binary EDT Module.bsl; test raw acquisition, caching and explicit strict recovery.
- [x] Preserve measured exchange-plan attribute membership and types for strict source validation.
- [ ] Preserve the legacy BUH defined-type identity in SDK full/header projections; test selective acquisition of the definition and concrete target.
- [ ] Repeat all six XML/EDT configured openings, preserving failures as failures until every initialization succeeds.
