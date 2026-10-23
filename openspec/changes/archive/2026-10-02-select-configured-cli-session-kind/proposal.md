## Why

The configured CLI always opens a thin-client session. A web-client startup
scenario cannot currently select the existing SessionKind through this host.

## What Changes

Add optional `--session-kind thin-client|web-client` to database script mode.
Preserve thin-client behavior when omitted. Actual background sessions remain
created through the parent-bound background API, retaining inherited parameters
and a separate connection; this switch does not launch a background job.
