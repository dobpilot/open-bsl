# Supply the CLI host connection snapshot

## Why
Original DEMO initialization reaches `GetInfoBaseConnections`, but the CLI does not supply its authorized connection snapshot. The user selected all real Session instances of the current open-bsl host as the scope.

## What Changes
Supply the complete host-local snapshot for the selected principal when its acquired roles grant ActiveUsers. The current CLI opens one configured Session, so its complete snapshot contains that actual connection. Identify the hosting application as `bsl-cli`, independently of its selected SessionKind. Do not query native 1C sessions or fabricate unavailable connection data.

## Impact
Configured CLI initialization only. Standalone execution and the existing immutable SessionResources snapshot API remain unchanged. Adding more configured Sessions to this CLI process will require including them in its snapshot.

Expose the already acquired configuration object identity through a read-only MetadataCatalog getter so host authorization uses the same root as existing runtime rights checks.
