# Business-process route points

Original demo manager code reads `ТочкиМаршрута` and compares task route
references with named configuration points. The SDK now acquires their full
composition. Expose these values through the existing ORM and session facade,
without modifying source text or introducing another execution pipeline.

The change adds public Rust metadata projection and BSL component members.
It does not implement business-process execution or change name precedence.
