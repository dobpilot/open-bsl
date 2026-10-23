# Match reached StrReplace coercion

## Why
Original demo passed the measured StrFind correction and reached StrReplace with an argument rejected by the current strict string implementation. Native controls establish conversion in all three positions.

## What Changes
Convert measured simple arguments using the existing formatter, with ungrouped decimal-point numeric formatting. Share conversion with StrFind and preserve existing UUID receiver support and unsupported-type handling.

## Impact
Only builtin execution in the VM changes. No new dependency, resolver change or public declaration.
