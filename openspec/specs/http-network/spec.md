# http-network Specification

## Purpose
Define the HTTP component's platform certificate-source contracts so original
configuration code can select verified TLS authorities through native BSL types.

## Requirements

### Requirement: Linux authority certificate source
The HTTP component SHALL resolve the zero-argument
`СертификатыУдостоверяющихЦентровLinux` and
`LinuxCertificationAuthorityCertificates` constructors to a separate native
type. On Linux, OpenSSL connections SHALL accept this object as a verified
system-root authority source. On unsupported operating systems construction
SHALL fail explicitly rather than use another platform's store.

#### Scenario: Native Linux identity
- **WHEN** either constructor name executes on Linux
- **THEN** the resulting type is `LinuxCertificationAuthorityCertificates`
- **AND** it differs from the OS authority-certificate type

#### Scenario: Secure connection authority
- **WHEN** an OpenSSL connection receives this object as its authority source on Linux
- **THEN** the transport uses system-root certificate verification
- **AND** certificate and hostname verification remain enabled

#### Scenario: Unavailable Linux store
- **WHEN** the constructor executes on an unsupported operating system
- **THEN** it reports an explicit unsupported-platform error
