# Declare information register factories

## Why
Original UNF manager code cannot compile because its standard zero-argument record factories are missing from the finite receiver context.

## What Changes
Declare CreateRecordManager and CreateRecordSet on information register managers, with their Russian-first aliases and documented arity. Until their real record implementation is available, executing either fails explicitly. This enables dormant original procedures to link without fabricating records or database writes.

## Impact
bsl-orm method descriptors, Session manager receiver contexts and generated API reference. Record implementations and database adapters remain incomplete work in the configuration session runtime change.
