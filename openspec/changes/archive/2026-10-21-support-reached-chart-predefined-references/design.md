# Design
Native 1C 8.3.27.2342 accepted an independent chart control and confirmed bare manager properties, Russian and English aliases, qualified ChartOfAccountsRef type lookup, query VALUE identity, and read-only manager properties. Evidence is retained under docs/research/fixtures/chart-predefined-manager. The predefined metadata GUID differs from the actual record UUID.

Reuse the existing immutable reference representation, configuration property declarations, and session-bound predefined query path. Add ChartOfAccounts to the existing owner-kind filters and use ПланСчетов in query VALUE expressions; the executable global remains ПланыСчетов. Preserve local and module variable precedence without changing core resolution.

Typed query reference materialization must accept the declared chart owner without treating it as a catalog. Use the shared configuration reference constructor with explicit supported-owner validation. No account object CRUD or accounting execution is added.

The separate completed native empty-ref control confirms both manager receiver aliases, nil UUID, reference type and IsEmpty aliases. Register only the reached zero-argument method, reusing document_empty_ref and the common immutable reference representation. GetRef and account CRUD remain outside this change.
