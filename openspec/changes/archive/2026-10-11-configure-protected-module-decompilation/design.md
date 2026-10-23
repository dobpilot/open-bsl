# Design

Expose `LoadOptions { decompile_protected_modules: bool }`, defaulting to false,
and option-bearing counterparts of the existing database code and factory
loaders. Existing public entry points use the default. Pass the option through
both base and extension resources; never use process-global state. The private
resource reader and native complete decoder remain available to existing
complete-body regression tests, independently of public loader defaults.

For an image-only container, false generates only measured exported procedure
and function signatures, using existing parameter/default formatting. It
omits private and external declarations, local/module variables and initializer.
Procedures have no statements; functions contain `Return True`. Unsupported
exported signatures or malformed image tables still fail. Body opcode support
is irrelevant to interface-only mode. A container containing source text uses
that text regardless of the option.

True invokes the existing strict decompiler and its conservative cleanup. It
does not fall back to placeholders on failure. This is a deliberate host policy,
not a model of native protected execution, so no inferred 1C behavior is claimed.

Read-only UNF and buh loading reaches an exported procedure with flag18.
The retained native method-flag2 control already measures exported flags18/19
and promise semantics. Interface placeholders deliberately emit synchronous
procedures/functions for those flags, consistent with the requested empty/True
contract; full-body recovery must not silently erase async behavior. Reuse the
existing declaration formatter without broadening complete-body acceptance.

The CLI boolean flag is valid only before the script path with database mode,
rejects repetition and attached values, and remains a script argument after the
path. Document the default's effect clearly. Do not edit existing tests to hide
regressions; retain complete-body controls and add independent default-policy
tests. Core runtime and VM dependencies remain unchanged.
