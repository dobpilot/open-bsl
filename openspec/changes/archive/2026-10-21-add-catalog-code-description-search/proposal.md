# Why
Original demo manager source calls FindByCode and exact FindByDescription. The ORM has no declarations or execution path for these calls. Native isolated controls establish ordinary string matching and prefix behavior.

# What Changes
Implement ordinary string-code and description searches using the existing principal-bound query adapter. Preserve existing manager exports and resolver priority. Reject unsupported modes explicitly; do not fabricate successful empty results on adapter failures.

# Impact
bsl-orm, its session integration tests and generated API documentation. No new dependencies, database driver or public adapter trait. Full hierarchical codes, numeric code conversion and parent/owner filters remain explicit implementation boundaries until measured.
