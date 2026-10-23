# Design

Native 8.3.27.2342 source and hand-assembled opcode-70 probes each produce
18 receipt lines. Numeric inputs retain their value and type; Boolean
inputs yield one or zero. Invalid strings, Undefined and Null raise
catchable errors. Reuse the existing number representation and existing
string parser. Do not add conversion semantics to the image decoder.
Fractional results are reported after multiplication by ten to avoid an
unrelated regional decimal separator comparison.
