# Change: Expose the live query parameter structure

## Why
Original demo reads Query.Parameters while locating the current user. The ORM keeps a separate vector and exposes no property. Native 1C confirms a live mutable Structure and a read-only property.

## What Changes
Expose Parameters/Параметры using the existing runtime Structure as the sole parameter storage. SetParameter and execution share that structure. Preserve the measured latest key spelling and insertion order.

## Impact
QueryValue storage, its property descriptor, query request snapshots, focused tests and generated API reference. No core/VM machinery or dependencies.
