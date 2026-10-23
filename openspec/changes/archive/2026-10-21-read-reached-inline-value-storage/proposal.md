# Why
After effective privileged access is honored, original UNF session initialization reaches a ValueStorage field. SQL returns 410 inline bytes; the existing SDK stored-value decoder recognizes its compressed internal serialization, but the host adapter rejects Binary cells.

# What Changes
Decode acquired inline ValueStorage fields through the existing SDK framing/decompression and existing runtime internal-value decoder, then construct the existing detached ValueStorage snapshot. Keep unknown binary columns and external stored-value references explicit failures until supported.

# Impact
bsl-config-db, bsl-orm, and a closed facade entry point to the existing internal decoder. SDK needs a text decoding entry point so no second framing decoder or tree serializer is introduced. Core runtime and VM gain no database dependency.
