# Design
The saved BUH native CF/XML pair confirms the class shape and module GUID/procedure fields across 185 jobs. A separate native file-base control confirms that the runtime string omits the XML CommonModule prefix. See docs/research/fixtures/scheduled-job-method-name-1015.

Resolve the explicit GUID against the referenced common-module description, independently of collection order. Use the existing SDK inflate and serialized parser. Preserve unknown class shapes without fabricating a property. A recognized link with a missing or invalid target fails acquisition. Add Russian and English aliases through the existing immutable string-property mechanism. This does not start scheduled jobs or add their execution to the scope.
