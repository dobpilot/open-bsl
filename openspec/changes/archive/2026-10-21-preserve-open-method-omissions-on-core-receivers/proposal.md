# Why
Original BUH compilation rejects `TextWriter.Open(path,,,True)` even though this method already uses open object dispatch. The resolver applies closed-core argument restrictions before the compiler selects that existing dispatch.

# What Changes
Preserve omitted positions for method names outside the closed builtin table, including statically known core receivers. Keep closed builtin restrictions and runtime missing-method errors.

# Impact
Resolver correction only; no new methods, transport, dependencies or VM instructions. Native evidence is retained in docs/research/fixtures/open-method-omissions-1016.
