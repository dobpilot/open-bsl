# Design

Retained native 1C 8.3.27.2342 control in docs/research/fixtures/extension-hash-type establishes String=false, BinaryData=true and Base64String(HashSum) equal to the acquired root-key Base64. The vendor help page HashSum10768.html declares String; measured runtime behavior takes precedence.

Keep ConfigurationExtension's acquired Base64 representation and public constructor signature. The BSL getter decodes it through the existing runtime decoder, exposed through open-bsl, and returns the existing BinaryData value. Invalid host-provided Base64 fails explicitly rather than returning an invented digest. No dependency, SQL or Base64String change is needed.

The existing configuration_extensions_filter_each_complete_source test concatenates a placeholder hash-one String. Its fixture and expression must be corrected to valid Base64 and explicit Base64String(HashSum). Prepare an independently reviewable patch; do not alter the test before required approval.
