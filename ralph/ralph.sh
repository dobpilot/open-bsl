#!/usr/bin/env bash
set -euo pipefail

# ── Ralph loop: plan → implement → review, with a review panel on hard tasks ─
#
#   plan       $PLAN_MODEL    (picks a task or plans the fix for a rejected one)
#   implement  $IMPL_MODEL
#   review     $REVIEW_MODEL                          ← always
#   review     $REVIEW2_MODEL  (independent, panel)   ← only when the task is [hard]
#   arbiter    bash    (PASS only if every active reviewer says PASS)
#
# The four models are knobs: `PLAN_MODEL=opus ./ralph/ralph.sh` swaps one phase
# without editing the script, which is what a per-model usage limit calls for.
# The two verdict files keep their names whatever model writes them — the
# arbiter and ralph/prompts/plan.md both address them by name.
#
# State lives on disk, not in any session's context:
#   TASKS.md          backlog; the arbiter ticks a task off only on PASS
#   PLAN.md           the current plan the phases hand off through
#   REVIEW_FABLE.md   the first reviewer's verdict for the last attempt (kept until a PASS)
#   REVIEW_OPUS.md    the second reviewer's verdict, hard tasks only
#   COMMIT_MSG.md     the implementer's commit message for the whole task
#   PROGRESS.md       append-only log you read afterwards
#
# History: a FAIL keeps its mechanical checkpoint so the next iteration can be
# bisected against it, and a PASS collapses the task's checkpoints into one
# commit written from COMMIT_MSG.md (see ralph/prompts/commit-style.md). The
# history therefore carries one commit per task, not one per iteration.
#
#   Run from repo root:   ./ralph/ralph.sh
#   Tune iterations:      MAX_ITERS=40 ./ralph/ralph.sh
# ────────────────────────────────────────────────────────────────────────────

MAX_ITERS="${MAX_ITERS:-20}"

# Model per phase. Панель на hard-задачах остаётся двухголосой даже когда обе
# роли достались одной модели: второй ревьюер приходит с чистым контекстом и не
# читает чужой вердикт, так что независимость прогона сохраняется — теряется
# только независимость самой модели.
PLAN_MODEL="${PLAN_MODEL:-opus}"
IMPL_MODEL="${IMPL_MODEL:-opus}"
REVIEW_MODEL="${REVIEW_MODEL:-opus}"
REVIEW2_MODEL="${REVIEW2_MODEL:-opus}"
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

# Where the current task started: walk back over its own iteration checkpoints
# and stop at the first commit that is not one of them. With no checkpoints yet
# this is HEAD, and the squash below degenerates into an ordinary commit.
task_base () {
  local slug="$1" cur="HEAD" base
  base="$(git rev-parse HEAD)"
  while git log -1 --format=%s "$cur" 2>/dev/null |
        grep -qE "^ralph: iter [0-9]+ ${slug} (PASS|FAIL)$"; do
    cur="$cur^"
    base="$(git rev-parse "$cur" 2>/dev/null)" || break
  done
  printf '%s\n' "$base"
}

for i in $(seq 1 "$MAX_ITERS"); do
  echo "═══════════════ iteration $i / $MAX_ITERS ═══════════════"

  # ── plan ───────────────────────────────────────────────────────────────────
  # If REVIEW_*.md with VERDICT: FAIL are still on disk, the last attempt was
  # rejected and the planner will plan the fix instead of picking a new task.
  echo "── [$PLAN_MODEL] plan ──"
  run "$PLAN_MODEL" "$PLAN_EFFORT" "$PLAN_TOOLS" "$(cat "$PROMPTS/plan.md")"

  if grep -q "$DONE_SENTINEL" PLAN.md 2>/dev/null; then
    echo "Planner reports no remaining tasks. Stopping."
    break
  fi

  TASK_ID="$(field TASK_ID)"
  DIFFICULTY="$(field DIFFICULTY)"
  echo "   task=${TASK_ID:-?}  difficulty=${DIFFICULTY:-normal}"

  # ── implement ──────────────────────────────────────────────────────────────
  echo "── [$IMPL_MODEL] implement ──"
  run "$IMPL_MODEL" "$IMPL_EFFORT" "$IMPL_TOOLS" "$(cat "$PROMPTS/implement.md")"

  # ── review ─────────────────────────────────────────────────────────────────
  rm -f REVIEW_FABLE.md REVIEW_OPUS.md

  echo "── [$REVIEW_MODEL] review ──"
  reff="$REVIEW_EFFORT"; [ "$DIFFICULTY" = "hard" ] && reff="$REVIEW_EFFORT_HARD"
  run "$REVIEW_MODEL" "$reff" "$REVIEW_TOOLS" \
"$(cat "$PROMPTS/review.md")

Write your entire verdict to the file REVIEW_FABLE.md and to no other file."

  PANEL=0
  if [ "$DIFFICULTY" = "hard" ]; then
    PANEL=1
    echo "── [$REVIEW2_MODEL] independent review (panel) ──"
    run "$REVIEW2_MODEL" "$REVIEW_OPUS_EFFORT" "$REVIEW_TOOLS" \
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

  TASK_MSG=""
  if [ "$PASS" -eq 1 ]; then
    [ -n "$TASK_ID" ] && sed -i "s/^- \[ \] (${TASK_ID})/- [x] (${TASK_ID})/" TASKS.md || true
    if [ -s COMMIT_MSG.md ]; then TASK_MSG="$(cat COMMIT_MSG.md)"; fi
    rm -f REVIEW_FABLE.md REVIEW_OPUS.md PLAN.md COMMIT_MSG.md
    log "PASS  ${TASK_ID}  panel=${PANEL}"
    echo "   ✔ PASS — task closed."
  else
    # Findings stay on disk; next iteration's planner reads them and plans the fix.
    log "FAIL  ${TASK_ID}  fable=${FABLE_V:-?} opus=${OPUS_V:-n/a}  panel=${PANEL}"
    echo "   ✘ FAIL — findings kept, retrying next iteration."
  fi

  # Checkpoint so the whole run is bisectable / revertable. A closed task
  # collapses into a single commit carrying the implementer's message; anything
  # else — a FAIL, or a PASS with no usable message — keeps the mechanical one.
  if ! git diff --quiet || ! git diff --cached --quiet; then
    git add -A
    SQUASHED=0
    if [ "$PASS" -eq 1 ] && [ -n "$TASK_MSG" ] && [ -n "$TASK_ID" ]; then
      BASE="$(task_base "$TASK_ID")"
      if [ -n "$BASE" ] && git merge-base --is-ancestor "$BASE" HEAD; then
        git reset --soft "$BASE"
        if printf '%s\n' "$TASK_MSG" | git commit -q -F -; then SQUASHED=1; fi
      fi
    fi
    if [ "$SQUASHED" -eq 0 ]; then
      git commit -q -m "ralph: iter $i ${TASK_ID:-?} $([ $PASS -eq 1 ] && echo PASS || echo FAIL)" || true
    fi
  fi
done

echo "Done. Read PROGRESS.md and the git log, then review the diff before merging."
