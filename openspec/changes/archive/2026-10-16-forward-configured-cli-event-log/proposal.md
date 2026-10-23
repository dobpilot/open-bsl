# Why
Original UNF initializes through SessionFactory with an attached host event writer. The configured CLI supplies no writer, so the same reached independent WriteLogEvent cannot complete there. The CLI must provide a concrete host destination to run that original initialization.

# What Changes
Attach a stderr event writer to configured CLI sessions. Preserve event name, level, comment, metadata and data using existing value formatting. Support independent delivery only; transactional delivery and event-use configuration remain explicit unsupported operations.

# Impact
CLI host wiring and tests. No database writes, new external dependencies or standalone CLI behavior changes.
