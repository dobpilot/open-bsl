# Design

Native 1C 8.3.27.2342 measure-catalog-predefined-flag.bsl yields false during the body and on a new object, true after assigning the acquired valid predefined name, and false after clearing it. Assignment to the flag is rejected and retains state. The English alias matches.

Use the existing predefined_name state; do not duplicate a mutable boolean. Register a read-only property in the existing finite receiver table so bare module access uses the current instance. Existing predefined-name validation remains unchanged. This does not implement loading persisted catalog objects.

The native eight-line property oracle was replayed unchanged, and existing predefined-name tests passed. Original demo passed this compile barrier and stopped at НаименованиеЯзык1 after 56.2 seconds without reaching the user script. The first workspace run hit a scratch-directory AlreadyExists failure in an existing launcher test; the unchanged focused retry passed. The full retry passed all workspace gates on 2026-10-02 (tests 339.4 seconds), including rustdoc, facade consumer, normal dependency trees and strict validation. Log: /tmp/open-bsl-predefined-flag-096-gates-retry.log.
