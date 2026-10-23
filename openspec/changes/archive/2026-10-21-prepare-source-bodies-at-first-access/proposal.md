# Prepare configured source bodies at first access

## Why
The factory profile prepares 4,727 BUH bodies before session startup because a static reference in any method recursively selects its target body. Native 1C 8.3.27.2342 successfully calls a valid parent export while an unused sibling references a child with an invalid constructor. The child compilation fails only at the actual child call. Existing transitive-module native controls record the same boundary.

## What Changes
- Keep configured module declarations available for ordinary semantic linking and verification.
- Prepare source-backed module bodies at their actual access, without recursively compiling referenced bodies merely for verification.
- Validate each real program through the existing compiler and verifier before publication, including agreement with its declaration surface.
- Preserve explicit roots/imports, legacy in-memory recipe behavior, name precedence, global export conflict checks, session isolation and protected-export stub policy.

## Impact
Private facade implementation and configured-session preparation behavior; no new language, public Rust API, bytecode format or dependency. Measure factory and full cold startup separately: delaying identical work is not sufficient evidence of a useful optimization. Existing test changes require a reviewable patch and user approval.
