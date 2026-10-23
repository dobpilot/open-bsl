# Support reached fixed map iteration

## Why
Original UNF startup traverses a FixedMap. Its public Count method exists, but the collection traversal protocol still rejects it. A native control confirms key/value traversal of the immutable snapshot.

## What Changes
Implement traversal through the existing collection protocol and reuse the existing key/value pair representation. Keep public indexing and mutation behavior unchanged.

## Impact
One runtime object gains internal traversal operations. No opcode, public method, constructor or dependency is added.
