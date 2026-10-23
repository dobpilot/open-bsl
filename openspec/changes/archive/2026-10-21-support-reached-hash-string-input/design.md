# Design

Feed the existing hasher directly from UTF-8 string bytes. Native ASCII and Cyrillic controls match explicitly encoded binary input. Reuse the existing binary path for other supported inputs; do not stringify arbitrary values. The original diagnostic prints only the argument type, never its value.
