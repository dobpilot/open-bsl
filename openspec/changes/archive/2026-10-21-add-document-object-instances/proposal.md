# Add document object instances

## Why
Original UNF document manager code calls CreateDocument, Fill and assigns Date. Document references alone cannot execute that code. Native initialization measurements now establish the object-module boundary instead of copying catalog assumptions.

## What Changes
Implement document creation and concrete object-module execution using the existing component protocol and configuration instances. Acquire document standard settings and typed attributes from the configuration source, and preserve unknown acquisition states. Add Russian and English method and type names through the facade.

## Impact
ORM document managers and objects, configuration metadata acquisition, facade API reference, session integration tests and native measurement fixtures. No new core/VM dependencies or alternate language execution path. Database persistence is not implied by in-memory creation.
