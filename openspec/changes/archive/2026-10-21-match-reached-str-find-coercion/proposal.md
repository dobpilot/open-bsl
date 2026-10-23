# Match reached StrFind coercion

## Why
Original demo passes Undefined as the first argument from its parameter substitution routine. Native 1C returns zero; the interpreter currently raises a string type error. Native empty-needle behavior also differs.

## What Changes
Use existing runtime string formatting for StrFind inputs and return one for an empty needle, preserving measured non-empty search controls.

## Impact
Existing builtin execution and search implementation only. No new public declarations, resolver behavior or dependencies.
