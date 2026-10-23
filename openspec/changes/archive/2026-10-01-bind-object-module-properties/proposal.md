# Bind object module properties

## Why
Original catalog object modules refer to their receiver properties without an
explicit ThisObject qualifier. UNF's VersionsExtensions object module currently
fails to compile at ExchangeData. The existing finite receiver method context
does not provide the corresponding property context.

## What Changes
- Add a finite, host-declared receiver property context for configuration modules.
- Resolve supported unqualified property reads and writes through the existing
  ThisObject receiver and property instructions.
- Carry that context into dynamic compilation and immutable worker recipes.
- Implement the measured additional properties and data exchange settings of a
  newly created catalog object without database writes.

## Impact
Public facade API and semantic resolution for explicitly configured modules;
catalog object component properties. Standalone resolution, library global name
order and runtime/VM dependencies remain unchanged.
