# Declare the reached server administration constructor

## Why
Original buh acquisition now succeeds but compiling the conservative common-module closure fails on New ServerAdministration in АдминистрированиеКластераRAS before session startup reaches that function. Native evidence confirms the constructor's two optional arguments and immediate ras connection attempt.

## What Changes
Register the Russian and English constructor names with arity zero through two in the existing session library. Compilation uses the existing registry. Execution fails explicitly because a ras transport is not implemented; it must not return a fabricated administration object or claim a successful connection.

## Impact
Public BSL constructor declaration and generated API reference. No new dependencies, resolver policies, bytecode changes or ras administration operations. Actual ras transport remains incomplete if startup reaches this constructor.
