# Support the reached current-user UUID

## Why
Original demo startup reads InfoBaseUser.УникальныйИдентификатор in СвойстваТекущегоПользователяИБ. The runtime user currently exposes its name and data separation only.

## What Changes
Expose the read-only УникальныйИдентификатор / UUID property from the acquired immutable Principal identifier, returning the existing UUID value type. Keep user editing and lookup APIs outside this change.

## Impact
One additive BSL property; no SDK acquisition changes or dependencies.
