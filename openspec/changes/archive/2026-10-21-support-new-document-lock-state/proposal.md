# Why
Original BUH reaches bare IsLocked() in Document.RegulatedOperation.ObjectModule after Modified binding succeeds. The current ORM supports only new document objects and has no object-lock operation.

# What Changes
Expose measured zero-argument IsLocked/Заблокирован for supported new document objects. Return False after selection; retain the selected-object error during object-module body initialization. Reuse the document receiver table and existing selection guard.

# Impact
One ORM method, focused native-backed tests and generated API. No persistent document loading, locking service, resolver changes or dependencies.
