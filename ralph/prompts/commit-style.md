Commit message style for this repository.

The loop squashes every iteration of a task into ONE commit and uses
COMMIT_MSG.md as its message, so that file is the permanent record of the
work — the iteration checkpoints disappear. Write it accordingly.

Shape:
- Subject: imperative mood, one line, roughly 50-72 characters,
  capitalized, NO trailing period. "Add the «ДвоичныеДанные» type with
  split and combine", not "Added binary data support." or "bin-binary-data".
- Blank line, then the body wrapped at 72 columns. Three to five dense
  paragraphs is the usual size; do not pad and do not compress into
  bullets.
- No trailers of any kind: no Co-Authored-By, no "Generated with", no
  emoji, no issue-tracker noise.

Language:
- English prose, like the rest of the history. Russian identifiers and
  platform terms keep their original form and go in «guillemets» inside
  English text: «Попытка», «ДвоичныеДанные», «ЗначениеЗаполнено».
- Identifiers, file names and opcodes are written plainly, without
  backticks: bundle.rs, Chunk::bundle_len, FORMAT_VERSION, NewBinaryData.

Content — this is the part that matters:
- Explain the MECHANISM and the INVARIANTS, not a list of touched files.
  What the new machinery does, which rules it must obey, which
  representation was chosen and what the alternative would have broken.
- State the non-obvious tradeoffs and say why they were taken. A reader
  six months out needs the reasoning, not the diff, which they already
  have.
- If the change threads a new opcode or builtin through the
  single-source tables (write_instr/parse_instr and OPCODES in text.rs,
  the round-trip corpus, the effects classification in bundle.rs,
  FORMAT_VERSION, the name tables in builtin.rs), say so once — it tells
  the reader the discipline was followed without listing each table.
- Measured platform behavior is quoted concretely, with the values:
  which spellings 8.3.27 accepted and which it rejected, what it
  actually returned, where a boundary sits. "Measured on the platform"
  without the value is worthless.
- One sentence on what was deliberately left unmeasured, and on any
  НЕ ИЗМЕРЕНО marker added or retired.
- Every claim must be checkable against the diff, the fixtures or
  PROGRESS.md. Never invent a number, a measurement or a file name.

Never mention the loop that produced the work: no iterations, no
reviewers, no verdicts, no "attempt", no task slug. The message reads as
if one author did the work in one sitting.

A message in this style, kept short here, from the existing history:

    Group the bytecode into VLIW bundles and dispatch them whole

    bundle.rs proves runs of neighboring instructions mutually
    independent and records the widths in Chunk::bundle_len. Packet
    semantics are classic VLIW — all reads before all writes: RAW and
    WAW are forbidden inside a bundle, WAR is allowed, since without
    that the LIFO temp reuse would break bundles between any two
    statements. [...] Interleaved same-sitting medians against the
    pre-bundle binary: pi_leibniz 471 -> 376 ms, table_total 116 -> 92.
    The one tax is call_overhead 135 -> 144 ms: the width fetch does not
    pay for itself where every step is a single instruction around a
    call.
