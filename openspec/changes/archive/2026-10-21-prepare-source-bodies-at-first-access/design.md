# Design

## Evidence and scope
`docs/research/fixtures/session-factory-hotspot-1021/native-transitive-body/receipt.json` records native success for the unused transitive body and a registration-journal compiler error for the called child. Existing `measure-module-transitive-arguments.compiler.platform.txt` records parent and argument markers before child compilation failure. No new platform rule is inferred.

The current static closure remains the behavior of eager in-memory recipes. Source-backed catalogs keep immutable declaration discovery and prepare explicit roots; static references discover and validate declarations without preparing their bodies. Runtime access uses the existing `ExecutionCatalog::module_code` path.

## Verification without executable substitutes
A private verifier-only catalog provides the signatures required by the existing link verifier. It is never attached to a VM. Its declaration structures contain export identities, variable slots, parameter modes and counts, not executable method bodies. Real code is produced by the ordinary parser, sema, compiler and verifier, and checked against the declared surface before publication. Runtime catalogs never return declaration-only structures as executable code. Existing initialization and native deferred compilation error mechanisms stay in force.

## Boundaries and priorities
No change to lookup precedence, facade dependencies, RT/VM dependency trees, BSL builtins, protected module decompilation defaults or serialized formats. Existing explicit imports remain preparation roots as before. The configuration namespace and module IDs remain stable; newly prepared code must tolerate previous runtime field-name growth.

## Validation and risk
Add source-load counters, unused/called child cases, signature mismatch rejection, import cycles, exported variable by-reference behavior and facade-only consumers. Review any old test whose assumption was eager transitive body acquisition before changing it. Compare alternating immutable release A/B runs on BUH and inspect total startup, instructions, body counts and RAM. A factory-only gain obtained by shifting equal work into session startup does not meet the performance objective.
