# Design

The scope is an explicit host policy chosen by the user, not an inference about native 1C cluster discovery. The host is the current CLI process. It opens exactly one configured Session; metadata acquisition connections and VM worker threads are not Session instances. Its complete snapshot therefore contains the configured connection numbered 1, as previously authorized. A single-entry complete host list is not a fallback for an unavailable larger list.

ApplicationName is `bsl-cli` for both thin-client and web-client execution contexts: the executable remains the same and does not become a native WebServerExtension. ComputerName is the current host name, acquired by the host; failure to acquire it fails explicitly rather than inventing a name. No new public API or registry is needed for the current single-Session CLI lifecycle.

Authorization uses the selected principal's already acquired role resources and the configuration identity. Missing or unreadable rights do not grant access. Without ActiveUsers, the CLI omits the snapshot and the existing runtime reports explicit unavailability on access. The privileged initialization frame does not expand the host-supplied principal capability.

Other hosts, including the diagnostic parent/background host, must supply their own complete snapshots of real Session instances. A child snapshot includes the still-live parent and child, with separate connection numbers. No native 1C connections are discovered, and no cross-process or database-global registry is introduced. Existing Sessions retain their immutable startup snapshots as specified by the current API.
