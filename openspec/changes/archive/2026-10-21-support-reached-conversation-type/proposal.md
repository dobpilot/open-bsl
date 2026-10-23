# Support reached conversation identifier type lookup

## Why
Original UNF initialization reaches the conversation identifier type lookup. Its constructor already resolves, but the session library does not register its type. Native controls confirm type lookup without a collaboration service.

## What Changes
Register the measured Russian and English names and presentation. Preserve the existing explicit constructor failure without a host implementation.

## Impact
One type descriptor is registered; no collaboration service, fabricated values or dependencies are added.
