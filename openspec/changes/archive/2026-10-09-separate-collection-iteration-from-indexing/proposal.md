# Separate collection iteration from public indexing

## Why
The retained native CommonAttributeContent control proves that a collection can support For Each while rejecting numeric indexing and Get. The compiler currently implements For Each through public GetIndex, so a host cannot faithfully expose that measured collection through the existing object protocol.

## What Changes
- Add one object-protocol operation for retrieving an element during iteration, defaulting to the existing indexed operation.
- Route compiled For Each through that operation while keeping ordinary brackets unchanged.
- Represent iteration access distinctly in the existing bytecode and its textual format.

## Impact
This extends the current compiler/VM/facade object protocol and does not add a second iterator executor, database dependency, or configuration-specific logic to the interpreter. Existing indexed collections retain their current behavior. The acquired CommonAttributeContent surface remains part of complete-configuration-session-runtime and depends on this extension.
