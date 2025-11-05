#!/bin/bash
# Rust Dependency Validation Script
# This script validates Rust dependencies and checks for common issues

set -e

echo "==================================="
echo "Rust Dependency Validation"
echo "==================================="
echo ""

cd "$(dirname "$0")"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if Rust and Cargo are installed
echo "1. Checking Rust installation..."
if ! command -v rustc &> /dev/null; then
    echo -e "${RED}✗ Rust is not installed${NC}"
    exit 1
fi
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ Cargo is not installed${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Rust $(rustc --version)${NC}"
echo -e "${GREEN}✓ Cargo $(cargo --version)${NC}"
echo ""

# Check dependencies compile
echo "2. Running cargo check..."
if cargo check 2>&1; then
    echo -e "${GREEN}✓ All dependencies compile successfully${NC}"
else
    echo -e "${RED}✗ Cargo check failed${NC}"
    exit 1
fi
echo ""

# Show dependency tree
echo "3. Dependency tree (top-level only):"
cargo tree --depth 1 || echo -e "${YELLOW}⚠ Could not generate dependency tree${NC}"
echo ""

# Check for unused dependencies (requires cargo-udeps, optional)
echo "4. Checking for unused dependencies..."
if command -v cargo-udeps &> /dev/null; then
    echo "Running cargo-udeps..."
    cargo +nightly udeps || echo -e "${YELLOW}⚠ Some unused dependencies detected${NC}"
else
    echo -e "${YELLOW}⚠ cargo-udeps not installed (optional)${NC}"
    echo "   Install with: cargo install cargo-udeps --locked"
fi
echo ""

# Security audit (requires cargo-audit, optional)
echo "5. Security audit..."
if command -v cargo-audit &> /dev/null; then
    echo "Running cargo audit..."
    cargo audit || echo -e "${YELLOW}⚠ Some security advisories found${NC}"
else
    echo -e "${YELLOW}⚠ cargo-audit not installed (optional)${NC}"
    echo "   Install with: cargo install cargo-audit --locked"
fi
echo ""

# Check for outdated dependencies (optional)
echo "6. Checking for outdated dependencies..."
if command -v cargo-outdated &> /dev/null; then
    echo "Running cargo outdated..."
    cargo outdated || echo -e "${YELLOW}⚠ Some dependencies are outdated${NC}"
else
    echo -e "${YELLOW}⚠ cargo-outdated not installed (optional)${NC}"
    echo "   Install with: cargo install cargo-outdated --locked"
fi
echo ""

echo "==================================="
echo -e "${GREEN}Validation Complete!${NC}"
echo "==================================="
