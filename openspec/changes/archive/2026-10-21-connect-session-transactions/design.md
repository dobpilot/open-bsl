# Decisions

The adapter retains a nesting count and a cancellation flag alongside its already owned connection. The outer begin opens a real SDK read-only transaction. Inner completion does not commit that connection. Inner rollback makes data reads unavailable until outer completion; the outer commit then delegates to real rollback. Empty completion fails. Actual connection state is authoritative; SDK errors must not leave a fictitious active transaction.

Native evidence: measure-session-transaction-nesting.platform.txt (19 lines), measure-session-transaction-rollback-data.platform.txt (8 lines), and docs/research/session-transaction-nesting.md. These controls use own file databases and do not establish backend locking or isolation equivalence. SDK guarantees connection identity and read-only enforcement separately. No writes to original databases are authorized.

No new transport, second connection, SQL through query to emulate BEGIN, or callback capability is introduced. Transaction operations validate the frozen principal, and ordinary reads reject a cancelled transaction. Existing unsupported adapters keep their defaults.
