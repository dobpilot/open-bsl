# Change: Support reached chart label locations

## Why
Original BUH manager code uses ChartLabelLocation.EdgeAuto, ChartScaleLabelLocation.None and, at the next reached manager, ChartLabelLocation.EdgeInside. Missing members prevent ordinary compilation of the acquired original source.

## What Changes
Declare only these measured reached members, Russian and English aliases, type identities and presentations through existing system enum tables. Keep the existing members intact.

## Impact
Runtime enum declarations, focused facade tests and regenerated API reference. No chart rendering, new dependency, resolver priority change or source rewrite.
