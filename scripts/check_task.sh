#!/usr/bin/env bash
#
# check_task.sh — MVP-focused mechanical gates.
#
# This script intentionally keeps only the highest-value, lowest-overhead
# runtime guards for the current wallet MVP path.
#
# Scope:
#   - phase1: signer/session command registration is wired into Tauri
#   - phase2: SQLite schema changes remain additive-only
#   - phase4: dashboard and BTC price-state commands are wired into Tauri
#   - phase5: run the current MVP gate set required by testing-strategy.md
#   - all: run every active MVP gate
#
# Reference:
#   docs/architecture/executable-wallet-runtime-blueprint.md
#
# This script no longer enforces the earlier 5-phase wallet-foundation
# hardening contract. It is intentionally narrowed to MVP runtime safety checks.
#
# Usage:
#   scripts/check_task.sh [phase1|phase2|phase4|phase5|all]
#
# Exits 0 if all gates pass, non-zero on any failure.

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIB_RS="$PROJECT_ROOT/src-tauri/src/lib.rs"
PHASE="${1:-all}"
fail_count=0

say()  { printf '\033[1;36m[check]\033[0m %s\n' "$*"; }
pass() { printf '  \033[1;32mPASS\033[0m %s\n' "$*"; }
fail() { printf '  \033[1;31mFAIL\033[0m %s\n' "$*"; fail_count=$((fail_count + 1)); }

# -----------------------------------------------------------------------------
# Gate 4: invoke_handler! command registration
# MVP-oriented checks only. This gate validates that the minimal user-facing
# runtime commands required by the current MVP path are actually wired into
# Tauri's invoke handler.
# -----------------------------------------------------------------------------
check_gate4() {
  say "Gate 4: invoke_handler! command registration"
  if [ ! -f "$LIB_RS" ]; then
    fail "lib.rs not found at $LIB_RS"
    return
  fi

  local required=()
  case "$PHASE" in
    phase1) required+=(security_unlock security_lock security_is_unlocked) ;;
    phase4|phase5|all) required+=(get_dashboard_stats refresh_dashboard_stats state_get_bitcoin_price_state) ;;
  esac

  if [ ${#required[@]} -eq 0 ]; then
    pass "no command-registration requirements for phase $PHASE"
    return
  fi

  # Extract registered command names from the first generate_handler! block.
  local registered_names
  registered_names=$(awk '
    /generate_handler!\[/ { in_block = 1; next }
    in_block && /^[[:space:]]*\]\)/ { in_block = 0; next }
    in_block {
      if ($0 ~ /^[[:space:]]*\/\//) {
        next
      }
      if ($0 ~ /^[[:space:]]*[A-Za-z0-9_:]+,[[:space:]]*$/) {
        line = $0
        gsub(/^[[:space:]]+/, "", line)
        gsub(/,[[:space:]]*$/, "", line)
        count = split(line, parts, /::/)
        print parts[count]
      }
    }
  ' "$LIB_RS" | sort -u)

  if [ -z "$registered_names" ]; then
    fail "unable to extract command names from invoke_handler!"
    return
  fi

  for cmd in "${required[@]}"; do
    if printf '%s\n' "$registered_names" | grep -qx "$cmd"; then
      pass "invoke_handler! registers $cmd"
    else
      fail "invoke_handler! missing $cmd (required for phase $PHASE)"
    fi
  done
}

# -----------------------------------------------------------------------------
# Gate 8: Additive-only SQL migration
# Purpose: enforce the MVP rule that SQLite schema changes stay additive-only.
# -----------------------------------------------------------------------------
check_gate8() {
  say "Gate 8: db.rs SQL changes must be additive"
  local db_file="$PROJECT_ROOT/src-tauri/src/db.rs"
  local db_rel="src-tauri/src/db.rs"

  if [ ! -f "$db_file" ]; then
    fail "db.rs missing at $db_file"
    return
  fi

  if git -C "$PROJECT_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1 && git -C "$PROJECT_ROOT" rev-parse --verify main >/dev/null 2>&1; then
    local added_lines
    added_lines=$(git -C "$PROJECT_ROOT" diff --unified=0 main -- "$db_rel" | grep '^+' | grep -v '^+++' || true)

    if [ -z "$added_lines" ]; then
      pass "db.rs has no added SQL drift against main"
      return
    fi

    local violations=""
    local drop_hits rename_hits alter_drop_hits add_not_null_hits
    drop_hits=$(printf '%s\n' "$added_lines" | grep -niE 'DROP[[:space:]]+(TABLE|COLUMN)' || true)
    rename_hits=$(printf '%s\n' "$added_lines" | grep -niE 'ALTER[[:space:]]+TABLE.*RENAME' || true)
    alter_drop_hits=$(printf '%s\n' "$added_lines" | grep -niE 'ALTER[[:space:]]+TABLE.*DROP' || true)
    add_not_null_hits=$(printf '%s\n' "$added_lines" | awk '
      BEGIN { IGNORECASE = 1 }
      /ALTER[[:space:]]+TABLE/ && /ADD[[:space:]]+COLUMN/ && /NOT[[:space:]]+NULL/ && $0 !~ /DEFAULT/ { print NR ":" $0 }
    ' || true)

    [ -n "$drop_hits" ] && violations="${violations}"$'\n'"forbidden DROP in added db.rs lines: $drop_hits"
    [ -n "$rename_hits" ] && violations="${violations}"$'\n'"forbidden ALTER TABLE ... RENAME in added db.rs lines: $rename_hits"
    [ -n "$alter_drop_hits" ] && violations="${violations}"$'\n'"forbidden ALTER TABLE ... DROP in added db.rs lines: $alter_drop_hits"
    [ -n "$add_not_null_hits" ] && violations="${violations}"$'\n'"forbidden ALTER TABLE ADD COLUMN NOT NULL without DEFAULT: $add_not_null_hits"

    if [ -z "$violations" ]; then
      pass "db.rs added SQL is additive-only against main"
    else
      while IFS= read -r line; do
        [ -n "$line" ] && fail "$line"
      done <<< "$violations"
    fi
    return
  fi

  local fallback_hits
  fallback_hits=$(grep -niE 'DROP[[:space:]]+(TABLE|COLUMN)|ALTER[[:space:]]+TABLE.*RENAME|ALTER[[:space:]]+TABLE.*DROP' "$db_file" || true)
  if [ -n "$fallback_hits" ]; then
    pass "main ref missing; fallback db.rs scan found potentially non-additive SQL shapes: $fallback_hits"
  else
    pass "main ref missing; fallback db.rs scan found no forbidden SQL shapes"
  fi
}

# -----------------------------------------------------------------------------
# Runner
# -----------------------------------------------------------------------------
case "$PHASE" in
  phase1)
    check_gate4
    ;;
  phase2)
    check_gate8
    ;;
  phase4)
    check_gate4
    ;;
  phase5)
    check_gate4
    check_gate8
    ;;
  all)
    check_gate4
    check_gate8
    ;;
  *)
    echo "Usage: $0 [phase1|phase2|phase4|phase5|all]" >&2
    exit 2
    ;;
esac

echo
if [ "$fail_count" -eq 0 ]; then
  printf '\033[1;32mAll gates passed for phase %s.\033[0m\n' "$PHASE"
  exit 0
else
  printf '\033[1;31m%d check(s) failed for phase %s.\033[0m\n' "$fail_count" "$PHASE"
  exit 1
fi
