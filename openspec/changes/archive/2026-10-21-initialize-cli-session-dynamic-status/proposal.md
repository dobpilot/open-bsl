# Initialize the configured CLI session dynamic status

## Why
Original demo and UNF initialization reach the existing session-specific dynamic configuration function, but the CLI does not supply its host resource. A complete native control establishes false for a newly opened session, including one opened after a dynamic update; an older retained session independently reports true.

## What Changes
Supply the measured initial false state when the configured CLI opens its own new logical session. Keep absent-resource failure for other hosts. Do not add a fabricated current-database flag to SDK acquisition.

## Impact
Only configured CLI session resource construction changes. This does not implement observation or adoption of external dynamic updates during a running CLI session. No dependency, resolver priority or standalone behavior changes.
