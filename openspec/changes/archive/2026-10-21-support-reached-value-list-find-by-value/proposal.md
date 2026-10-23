# Support reached value list lookup

## Why
Original demo reaches ValueList.FindByValue during session initialization. The existing value list implementation lacks this documented method. Sixteen native control lines establish lookup behavior.

## What Changes
Add НайтиПоЗначению / FindByValue with one required argument to the existing component table. Return the first matching existing item or Undefined using existing value equality.

## Impact
One component method, no new VM instruction, global name, dependency or object model.
