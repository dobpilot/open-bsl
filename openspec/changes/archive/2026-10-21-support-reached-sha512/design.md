# Design

Use sha2::Sha512 through the same digest abstraction as SHA256. Retain current binary input conversion and snapshot semantics. Measure empty input, abc, incremental input and repeated snapshots on native 1C before implementation. Do not implement unrelated CRC32 or PBKDF2 algorithms.
