# Declare catalog manager template retrieval

## Why
The original UNF Catalog.Counterparties manager cannot link because its external-data loading procedure calls GetTemplate. Other manager kinds already have finite declarations for this platform method.

## What Changes
Declare the catalog manager method with Russian and English aliases and one required argument. Preserve an explicit failure until real template resources can be acquired and returned; do not fabricate a template.

## Impact
BSL ORM catalog manager context, API reference and session linking tests. No new dependencies or changes to standalone resolution.
