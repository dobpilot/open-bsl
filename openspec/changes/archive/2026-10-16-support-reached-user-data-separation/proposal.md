# Support reached user data separation snapshots

## Why
Original UNF startup reads CurrentUser().DataSeparation.Count(). The SDK acquisition is still pending, but the ORM must expose the measured property when a host supplies an acquired snapshot and must distinguish unknown data from known emptiness.

## What Changes
Allow Principal to retain an optional existing serialized value graph of the user's data separation structure. Expose the read property through existing user objects, materializing once per object and independently on each CurrentUser call. Do not infer or fabricate database values.

## Impact
One additive Rust builder method and one BSL read property. Existing user property write restrictions remain; no user persistence or new dependencies are introduced.
