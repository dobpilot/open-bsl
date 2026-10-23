# Declare the reached AccessToken constructor

## Why
Original BUH compilation reaches a cold AccessToken creation in an exported integration method. Native platform evidence establishes constructor arities 0/2 and the HS256 enumeration aliases. Compiling that method does not require implementing JWT.

## What Changes
Add a declaration-only constructor through the existing crypto component and the measured HS256 system enumeration value. Actual token creation fails explicitly. Preserve existing resolver priorities and dependencies.

## Impact
Public BSL constructor/type/enum declarations and generated reference change. No JWT properties, signing, serialization, network, authentication, SDK, or unrelated API changes.
