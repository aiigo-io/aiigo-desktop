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
# Runner
# -----------------------------------------------------------------------------
case "$PHASE" in
  skeleton)
    check_gate1
    check_gate2_3
    ;;
  phase1|phase2|phase3)
    check_gate1
    check_gate2_3
    check_gate4
    ;;
  phase4|all)
    check_gate1
    check_gate2_3
    check_gate4
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
