# Design

Platform 8.3.27.2342 native measurements in
`measure-linux-authority-certificates.platform.txt` confirm a distinct
`LinuxCertificationAuthorityCertificates` type, its English alias, and acceptance
as the second OpenSSL connection argument. The installed Russian syntax helper
page `platform-syntax-help:syntax-context:objects/catalog63/catalog578/catalog2014/LinuxCertificationAuthorityCertificates.html`
documents PEM certificates in `/etc/ssl/certs`; the OS certificate page states
that its Linux behavior is equivalent. These observations do not prove HTTPS
trust outcomes or eager-loading timing.

Use a separate object descriptor rather than aliasing the OS object's identity.
On Linux reuse the existing verified system-root transport configuration. On
other operating systems reject execution explicitly rather than substituting
another operating system's certificate store. No runtime or VM dependency is
added. No certificate parsing implementation is duplicated.
