# Expose measured qualified type-description properties

The native accumulation balance receipt cannot currently be replayed: column
type descriptions retain qualifiers internally but do not expose their string
and number qualifier properties. Their string presentation also differs.
Reuse the existing runtime qualifier objects and type-description representation;
do not introduce a separate ORM type model or change conversion rules.
