# Why
Original UNF background session initialization reads ScheduledJob.MethodName while enumerating configuration metadata. Database projection currently omits the acquired method link.

# What Changes
Project the measured scheduled-job method link into immutable metadata string properties using both BSL aliases. Reuse acquired resources and the existing SDK parser.

# Impact
Database metadata projection only. No scheduler, job execution, resolver changes, new SDK API or external dependencies.
