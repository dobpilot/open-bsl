#!/usr/bin/env bash
set -euo pipefail

# ── Ralph loop: fable → opus → fable, with a review panel on hard tasks ──────
#
#   plan       fable   (picks a task or plans the fix for a rejected one)
#   implement  opus
#   review     fable                         ← always
#   review     opus    (independent, panel)  ← only when the task is [hard]
#   arbiter    bash    (PASS only if every active reviewer says PASS)
#
# State lives on disk, not in any session's context:
#   TASKS.md          backlog; the arbiter ticks a task off only on PASS
#   PLAN.md           the current plan the phases hand off through
#   REVIEW_FABLE.md   Fable's verdict for the last attempt (kept until a PASS)
#   REVIEW_OPUS.md    Opus's verdict, hard tasks only
#   PROGRESS.md       append-only log you read afterwards
#
#   Run from repo root:   ./ralph/ralph.sh
#   Tune iterations:      MAX_ITERS=40 ./ralph/ralph.sh
# ────────────────────────────────────────────────────────────────────────────

MAX_ITERS="${MAX_ITERS:-20}"
PROMPTS="$(cd "$(dirname "$0")/prompts" && pwd)"
DONE_SENTINEL="ALL_TASKS_DONE"

# Least privilege per phase.
PLAN_TOOLS="Read,Grep,Glob,Edit,Write"
IMPL_TOOLS="Read,Grep,Glob,Edit,Write,Bash"
REVIEW_TOOLS="Read,Grep,Glob,Bash,Write"      # Write is for the verdict file only

# Reasoning effort per phase. The gate (review) gets the ceiling on hard tasks.
PLAN_EFFORT="xhigh"
IMPL_EFFORT="xhigh"
REVIEW_EFFORT="xhigh"
REVIEW_EFFORT_HARD="max"
REVIEW_OPUS_EFFORT="xhigh"                     # second reviewer, hard tasks only

[ -f PROGRESS.md ] || echo "# Progress log" > PROGRESS.md

run () {  # $1=model  $2=effort  $3=tools  $4=prompt-string
  claude -p "$4" --model "$1" --effort "$2" --allowedTools "$3"
  # Fully unattended in a sandbox? Replace --allowedTools with:
  #   --permission-mode bypassPermissions
}

log ()      { printf '%s  %s\n' "$(date -u +%FT%TZ)" "$1" >> PROGRESS.md; }
field ()    { sed -n "s/^## $1:[[:space:]]*//p" PLAN.md | head -1 | tr -d '[:space:]'; }
verdict ()  { [ -f "$1" ] || return 0; sed -n 's/^VERDICT:[[:space:]]*//p' "$1" | head -1 | tr -d '[:space:]'; }

for i in $(seq 1 "$MAX_ITERS"); do
  echo "═══════════════ iteration $i / $MAX_ITERS ═══════════════"

  # ── plan (fable) ───────────────────────────────────────────────────────────
  # If REVIEW_*.md with VERDICT: FAIL are still on disk, the last attempt was
  # rejected and the planner will plan the fix instead of picking a new task.
  echo "── [fable] plan ──"
  run fable "$PLAN_EFFORT" "$PLAN_TOOLS" "$(cat "$PROMPTS/plan.md")"

  if grep -q "$DONE_SENTINEL" PLAN.md 2>/dev/null; then
    echo "Planner reports no remaining tasks. Stopping."
    break
  fi

  TASK_ID="$(field TASK_ID)"
  DIFFICULTY="$(field DIFFICULTY)"
  echo "   task=${TASK_ID:-?}  difficulty=${DIFFICULTY:-normal}"

  # ── implement (opus) ───────────────────────────────────────────────────────
  echo "── [opus] implement ──"
  run opus "$IMPL_EFFORT" "$IMPL_TOOLS" "$(cat "$PROMPTS/implement.md")"

  # ── review ─────────────────────────────────────────────────────────────────
  rm -f REVIEW_FABLE.md REVIEW_OPUS.md

  echo "── [fable] review ──"
  reff="$REVIEW_EFFORT"; [ "$DIFFICULTY" = "hard" ] && reff="$REVIEW_EFFORT_HARD"
  run fable "$reff" "$REVIEW_TOOLS" \
"$(cat "$PROMPTS/review.md")

Write your entire verdict to the file REVIEW_FABLE.md and to no other file."

  PANEL=0
  if [ "$DIFFICULTY" = "hard" ]; then
    PANEL=1
    echo "── [opus] independent review (panel) ──"
    run opus "$REVIEW_OPUS_EFFORT" "$REVIEW_TOOLS" \
"$(cat "$PROMPTS/review.md")

Write your entire verdict to the file REVIEW_OPUS.md and to no other file.
Review independently. Do not read REVIEW_FABLE.md."
  fi

  # ── arbiter (deterministic; the only step that can mark a task done) ────────
  FABLE_V="$(verdict REVIEW_FABLE.md)"
  OPUS_V="$(verdict REVIEW_OPUS.md)"

  PASS=0
  if [ "$FABLE_V" = "PASS" ] && { [ "$PANEL" -eq 0 ] || [ "$OPUS_V" = "PASS" ]; }; then
    PASS=1
  fi

  if [ "$PASS" -eq 1 ]; then
    [ -n "$TASK_ID" ] && sed -i "s/^- \[ \] (${TASK_ID})/- [x] (${TASK_ID})/" TASKS.md || true
    rm -f REVIEW_FABLE.md REVIEW_OPUS.md PLAN.md
    log "PASS  ${TASK_ID}  panel=${PANEL}"
    echo "   ✔ PASS — task closed."
  else
    # Findings stay on disk; next iteration's planner reads them and plans the fix.
    log "FAIL  ${TASK_ID}  fable=${FABLE_V:-?} opus=${OPUS_V:-n/a}  panel=${PANEL}"
    echo "   ✘ FAIL — findings kept, retrying next iteration."
  fi

  # Checkpoint so the whole run is bisectable / revertable.
  if ! git diff --quiet || ! git diff --cached --quiet; then
    git add -A
    git commit -q -m "ralph: iter $i ${TASK_ID:-?} $([ $PASS -eq 1 ] && echo PASS || echo FAIL)" || true
  fi
done

echo "Done. Read PROGRESS.md and the git log, then review the diff before merging."
