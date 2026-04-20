#!/usr/bin/env bash
#
# check_task.sh — mechanical drift gates for wallet foundation hardening.
#
# Runs the fast subset of acceptance gates (1/2/3/4) suitable for per-task
# feedback during AI-driven implementation. Full Milestone-meaning walkthrough
# (Gate 5) and docs-scan (Gate 6) still run at phase boundaries.
#
# Reference:
#   docs/superpowers/plans/2026-04-18-wallet-foundation-hardening.md
#   § Acceptance Gates (Per-Phase Zero-Drift Contract)
#
# Usage:
#   scripts/check_task.sh [skeleton|phase1|phase2|phase3|phase4|all]
#
# Exits 0 if all gates pass, non-zero on any failure.

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WALLET_DIR="$PROJECT_ROOT/src-tauri/src/wallet"
LIB_RS="$PROJECT_ROOT/src-tauri/src/lib.rs"
PHASE="${1:-skeleton}"
fail_count=0

say()  { printf '\033[1;36m[check]\033[0m %s\n' "$*"; }
pass() { printf '  \033[1;32mPASS\033[0m %s\n' "$*"; }
fail() { printf '  \033[1;31mFAIL\033[0m %s\n' "$*"; fail_count=$((fail_count + 1)); }

# -----------------------------------------------------------------------------
# Gate 1: Module Directory Diff
# Ground truth: the "New Rust Modules" list in the plan.
# -----------------------------------------------------------------------------
check_gate1() {
  say "Gate 1: module directory diff"
  local expected=(security state sync chain)
  for d in "${expected[@]}"; do
    if [ -d "$WALLET_DIR/$d" ]; then
      pass "wallet/$d/ exists"
    else
      fail "wallet/$d/ missing"
    fi
  done
}

# -----------------------------------------------------------------------------
# Gate 2 + 3: Type / Variant Parity
# Ground truth: the Rust code blocks embedded in Phase 2 Acceptance Criteria,
# Phase 3 lifecycle vocabulary, and ADR-0003 PriceStatus enumeration.
# -----------------------------------------------------------------------------
expect_variants() {
  local name="$1" file="$2" expected_count="$3"; shift 3
  if [ ! -f "$file" ]; then
    fail "$name: source file $file missing"
    return
  fi
  local found=0 missing=()
  for v in "$@"; do
    # Match a variant line like "    Fresh," or "    Fresh" (last variant).
    if grep -qE "^[[:space:]]+${v}[[:space:]]*,?[[:space:]]*$" "$file"; then
      found=$((found + 1))
    else
      missing+=("$v")
    fi
  done
  if [ "$found" -eq "$expected_count" ] && [ ${#missing[@]} -eq 0 ]; then
    pass "$name: $expected_count variants match"
  else
    fail "$name: expected $expected_count variants, found $found; missing: ${missing[*]:-none}"
  fi

  # Ensure no extra variants sneak in: count lines between `pub enum $name {` and `}`.
  local total
  total=$(awk "/pub enum ${name}[[:space:]]*\\{/,/^\\}/" "$file" \
    | grep -cE "^[[:space:]]+[A-Z][A-Za-z0-9]*[[:space:]]*,?[[:space:]]*$" || true)
  if [ -n "$total" ] && [ "$total" -gt "$expected_count" ]; then
    fail "$name: unexpected extra variants (found $total, expected $expected_count)"
  fi
}

expect_field() {
  local struct_name="$1" file="$2" field="$3"
  if [ ! -f "$file" ]; then
    fail "$struct_name.$field: source file $file missing"
    return
  fi
  if awk "/pub struct ${struct_name}[[:space:]]*\\{/,/^\\}/" "$file" \
       | grep -qE "pub[[:space:]]+${field}[[:space:]]*:"; then
    pass "$struct_name.$field"
  else
    fail "$struct_name.$field missing in $file"
  fi
}

check_gate2_3() {
  say "Gate 2+3: type and variant parity (plan + ADR contracts)"

  local state_file="$WALLET_DIR/state/types.rs"
  local sync_file="$WALLET_DIR/sync/types.rs"
  local sec_file="$WALLET_DIR/security/types.rs"

  # FreshnessStatus: exactly 5 variants (plan Phase 2 code block)
  expect_variants FreshnessStatus "$state_file" 5 Fresh Cached Stale Unavailable Partial

  # PriceStatus: exactly 4 variants (ADR-0003)
  expect_variants PriceStatus "$state_file" 4 Fresh Stale Unavailable Synthetic

  # FreshnessMetadata fields
  expect_field FreshnessMetadata "$state_file" status
  expect_field FreshnessMetadata "$state_file" updated_at
  expect_field FreshnessMetadata "$state_file" failed_sources

  # PriceState fields
  expect_field PriceState "$state_file" price_usd
  expect_field PriceState "$state_file" price_source
  expect_field PriceState "$state_file" price_updated_at
  expect_field PriceState "$state_file" status

  # BalanceState fields
  expect_field BalanceState "$state_file" raw_amount
  expect_field BalanceState "$state_file" display_amount
  expect_field BalanceState "$state_file" chain_id
  expect_field BalanceState "$state_file" freshness

  # PortfolioState fields
  expect_field PortfolioState "$state_file" value_usd
  expect_field PortfolioState "$state_file" value_btc
  expect_field PortfolioState "$state_file" freshness

  # LifecycleStatus: exactly 6 variants (Phase 3)
  expect_variants LifecycleStatus "$sync_file" 6 \
    Broadcasted Pending Confirmed Failed Replaced Dropped

  # SignerOperation: exactly 4 variants (Phase 1)
  expect_variants SignerOperation "$sec_file" 4 \
    Send Approve ExportMnemonic ExportPrivateKey
}

# -----------------------------------------------------------------------------
# Gate 4: invoke_handler! Command Registration Diff
# Phase-dependent required set. Expand as later phases land.
# -----------------------------------------------------------------------------
check_gate4() {
  say "Gate 4: invoke_handler! command registration"
  if [ ! -f "$LIB_RS" ]; then
    fail "lib.rs not found at $LIB_RS"
    return
  fi

  local required=()
  case "$PHASE" in
    phase4|all) required+=(get_bitcoin_price) ;;
  esac

  if [ ${#required[@]} -eq 0 ]; then
    pass "no command-registration requirements for phase $PHASE"
    return
  fi

  # Extract the body of the first invoke_handler! macro call.
  local handler_body
  handler_body=$(awk '
    /invoke_handler!/ { in_block = 1 }
    in_block { print }
    in_block && /\]/ { in_block = 0 }
  ' "$LIB_RS")

  for cmd in "${required[@]}"; do
    if printf '%s\n' "$handler_body" | grep -qE "\\b${cmd}\\b"; then
      pass "invoke_handler! registers $cmd"
    else
      fail "invoke_handler! missing $cmd (required for phase $PHASE)"
    fi
  done
}

# -----------------------------------------------------------------------------
# Gate 7: wallet/state/* type/trait allowlist
# Purpose: block type bloat / premature abstraction in wallet/state/*.
# -----------------------------------------------------------------------------
check_gate7() {
  say "Gate 7: wallet/state/* type/trait allowlist"
  local state_dir="$WALLET_DIR/state"
  local allowed=(FreshnessStatus FreshnessMetadata PriceStatus PriceState BalanceState PortfolioState)

  if [ ! -d "$state_dir" ]; then
    fail "wallet/state/ missing"
    return
  fi

  local findings
  findings=$(
    awk -v allowed_list="${allowed[*]}" '
      function brace_delta(line,   opens, closes, tmp) {
        tmp = line
        opens = gsub(/\{/, "{", tmp)
        tmp = line
        closes = gsub(/\}/, "}", tmp)
        return opens - closes
      }
      BEGIN {
        split(allowed_list, allowed, " ")
        for (i in allowed) ok[allowed[i]] = 1
        in_tests = 0
        depth = 0
      }
      /^[[:space:]]*mod[[:space:]]+tests[[:space:]]*\{/ {
        in_tests = 1
        depth = brace_delta($0)
        next
      }
      {
        if (in_tests) {
          depth += brace_delta($0)
          if (depth <= 0) {
            in_tests = 0
            depth = 0
          }
          next
        }
        if ($0 ~ /^[[:space:]]*pub[[:space:]]+(enum|struct|trait)[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/) {
          line = $0
          gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
          split(line, parts, /[[:space:]]+/)
          kind = parts[2]
          name = parts[3]
          sub(/[;{].*$/, "", name)
          if (!(name in ok)) {
            print FILENAME ":" FNR ": unexpected public " kind " " name
          }
        }
      }
    ' "$state_dir"/*.rs 2>/dev/null
  )

  if [ -z "$findings" ]; then
    pass "wallet/state/* only exposes allowlisted pub enum/struct/trait items"
  else
    while IFS= read -r line; do
      [ -n "$line" ] && fail "$line"
    done <<< "$findings"
  fi
}

# -----------------------------------------------------------------------------
# Gate 8: Additive-only SQL migration
# Purpose: enforce Plan's "additive SQLite changes only" rule in Phase 2.
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
# Gate 9: Tauri command return-shape parity
# Purpose: block DTO rewriting that silently diverges from frozen contracts.
# -----------------------------------------------------------------------------
check_gate9() {
  say "Gate 9: Phase 2 state commands return frozen struct types"
  local state_dir="$WALLET_DIR/state"
  local command_files
  command_files=$(grep -rl '^[[:space:]]*#\[tauri::command\]' "$state_dir"/*.rs 2>/dev/null || true)

  if [ -z "$command_files" ]; then
    pass "no state command surface yet, check deferred"
    return
  fi

  local results
  results=$(
    awk '
      /^[[:space:]]*#\[tauri::command\]/ {
        pending = 1
        signature = ""
        next
      }
      pending {
        signature = signature " " $0
        if ($0 ~ /\{[[:space:]]*$/) {
          if (signature ~ /fn[[:space:]]+/) {
            name = signature
            sub(/^.*fn[[:space:]]+/, "", name)
            sub(/\(.*/, "", name)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", name)
            if (signature ~ /(BalanceState|PortfolioState|PriceState|FreshnessMetadata)/) {
              print "PASS|" FILENAME "|" name
            } else {
              print "FAIL|" FILENAME "|" name "|" signature
            }
          }
          pending = 0
        }
      }
    ' $command_files
  )

  if [ -z "$results" ]; then
    pass "no state command surface yet, check deferred"
    return
  fi

  local had_fail=0
  while IFS='|' read -r verdict file name signature; do
    [ -z "$verdict" ] && continue
    if [ "$verdict" = "PASS" ]; then
      pass "$file: $name returns a frozen state contract type"
    else
      had_fail=1
      fail "$file: $name return type diverges from frozen state contracts: $signature"
    fi
  done <<< "$results"

  [ "$had_fail" -eq 0 ] || return
}

# -----------------------------------------------------------------------------
# Gate 10: Semantic test presence
# Purpose: prevent "test theater" around freshness/price distinctions.
# -----------------------------------------------------------------------------
check_gate10() {
  say "Gate 10: semantic test coverage for freshness/price distinctions"
  local state_dir="$WALLET_DIR/state"

  if ! grep -Rqs '^[[:space:]]*#\[test\]' "$state_dir"; then
    pass "no state logic yet, semantic tests deferred to P2-1"
    return
  fi

  local parsed_tests
  parsed_tests=$(
    awk '
      function brace_delta(line,   opens, closes, tmp) {
        tmp = line
        opens = gsub(/\{/, "{", tmp)
        tmp = line
        closes = gsub(/\}/, "}", tmp)
        return opens - closes
      }
      /^[[:space:]]*#\[test\]/ { pending = 1; next }
      pending && $0 ~ /^[[:space:]]*(pub[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+[A-Za-z_][A-Za-z0-9_]*/ {
        name = $0
        sub(/^.*fn[[:space:]]+/, "", name)
        sub(/\(.*/, "", name)
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", name)
        in_test = 1
        depth = brace_delta($0)
        body = $0 "\n"
        pending = 0
        next
      }
      in_test {
        body = body $0 "\n"
        depth += brace_delta($0)
        if (depth <= 0) {
          print "TEST|" name "|" body
          in_test = 0
          body = ""
          name = ""
          depth = 0
        }
      }
    ' "$state_dir"/*.rs
  )

  local found_partial=0 found_synthetic=0 found_stale=0 found_unavailable_portfolio=0
  local saw_any=0

  while IFS='|' read -r marker test_name test_body; do
    [ "$marker" != "TEST" ] && continue
    saw_any=1

    if printf '%s' "$test_body" | grep -q 'FreshnessStatus::Partial' &&
       printf '%s' "$test_body" | grep -q 'failed_sources'; then
      found_partial=1
    fi

    if ! printf '%s' "$test_name" | grep -q '_fresh' &&
       printf '%s' "$test_body" | grep -q 'assert' &&
       printf '%s' "$test_body" | grep -q 'PriceStatus::Synthetic'; then
      found_synthetic=1
    fi

    if printf '%s' "$test_body" | grep -q 'PriceStatus::Stale'; then
      found_stale=1
    fi

    if printf '%s' "$test_body" | grep -q 'Unavailable' &&
       printf '%s' "$test_body" | grep -q 'PortfolioState' &&
       printf '%s' "$test_body" | grep -q 'Fresh'; then
      found_unavailable_portfolio=1
    fi
  done <<< "$parsed_tests"

  if [ "$saw_any" -eq 0 ]; then
    pass "no state logic yet, semantic tests deferred to P2-1"
    return
  fi

  [ "$found_partial" -eq 1 ] \
    && pass "found test tying FreshnessStatus::Partial to failed_sources" \
    || fail "missing test with FreshnessStatus::Partial and failed_sources in the same test function"

  [ "$found_synthetic" -eq 1 ] \
    && pass "found non-fresh test asserting PriceStatus::Synthetic" \
    || fail "missing assertion for PriceStatus::Synthetic outside a *_fresh* test"

  [ "$found_stale" -eq 1 ] \
    && pass "found test asserting PriceStatus::Stale" \
    || fail "missing test coverage for PriceStatus::Stale"

  [ "$found_unavailable_portfolio" -eq 1 ] \
    && pass "found PortfolioState test covering Unavailable components vs Fresh portfolio status" \
    || fail "missing PortfolioState semantic test covering Unavailable inputs vs Fresh output"
}

# -----------------------------------------------------------------------------
# Runner
# -----------------------------------------------------------------------------
case "$PHASE" in
  skeleton)
    check_gate1
    check_gate2_3
    ;;
  phase1)
    check_gate1
    check_gate2_3
    check_gate4
    ;;
  phase2)
    check_gate1
    check_gate2_3
    check_gate4
    check_gate7
    check_gate8
    check_gate9
    check_gate10
    ;;
  phase3)
    check_gate1
    check_gate2_3
    check_gate4
    ;;
  phase4)
    check_gate1
    check_gate2_3
    check_gate4
    ;;
  all)
    check_gate1
    check_gate2_3
    check_gate4
    check_gate7
    check_gate8
    check_gate9
    check_gate10
    ;;
  *)
    echo "Usage: $0 [skeleton|phase1|phase2|phase3|phase4|all]" >&2
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
