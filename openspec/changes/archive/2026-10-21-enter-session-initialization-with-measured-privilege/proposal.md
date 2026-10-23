# Enter session initialization with measured privilege

Original session initialization must execute in the platform context, rather than merely call the correct procedure. A native managed-application control on platform 8.3.27.2342 observes privileged mode in both automatic initial and selective SessionModule entries, with ordinary mode restored in the caller. The current SessionFactory and lazy parameter path use ordinary handler entry.

Add an explicit per-call handler entry mode to the existing execution path and use it for automatic session parameter initialization. Do not infer module identity from names in the VM, change user roles, alter generic handler defaults, add a session-wide privilege flag, or implement common-module metadata privilege in this change.
