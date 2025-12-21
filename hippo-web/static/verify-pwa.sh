#!/bin/bash
# PWA Setup Verification Script
# Checks that all required files are in place

set -e

echo "🦛 Hippo PWA Setup Verification"
echo "================================"
echo ""

ERRORS=0
WARNINGS=0

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check functions
check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✓${NC} Found: $1"
    else
        echo -e "${RED}✗${NC} Missing: $1"
        ((ERRORS++))
    fi
}

check_file_warn() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✓${NC} Found: $1"
    else
        echo -e "${YELLOW}⚠${NC} Missing (optional): $1"
        ((WARNINGS++))
    fi
}

# Check core PWA files
echo "Checking Core PWA Files:"
echo "------------------------"
check_file "manifest.json"
check_file "sw.js"
check_file "index.html"
check_file "offline.html"
check_file "browserconfig.xml"
echo ""

# Check utility files
echo "Checking Utility Files:"
echo "-----------------------"
check_file "pwa-utils.js"
check_file "pwa-test.html"
echo ""

# Check documentation
echo "Checking Documentation:"
echo "-----------------------"
check_file "PWA_README.md"
check_file "SETUP.md"
echo ""

# Check icon source
echo "Checking Icon Files:"
echo "--------------------"
check_file "icons/icon.svg"
check_file "icons/generate-icons.sh"
check_file "icons/README.md"

# Check generated icons (warnings only)
check_file_warn "icons/icon-192.png"
check_file_warn "icons/icon-512.png"
check_file_warn "icons/apple-touch-icon.png"
check_file_warn "icons/favicon.ico"
check_file_warn "icons/favicon-16.png"
check_file_warn "icons/favicon-32.png"
echo ""

# Check manifest.json validity
echo "Checking manifest.json validity:"
echo "--------------------------------"
if [ -f "manifest.json" ]; then
    if command -v jq &> /dev/null; then
        if jq empty manifest.json 2>/dev/null; then
            echo -e "${GREEN}✓${NC} manifest.json is valid JSON"
        else
            echo -e "${RED}✗${NC} manifest.json has syntax errors"
            ((ERRORS++))
        fi
    else
        echo -e "${YELLOW}⚠${NC} jq not installed, skipping JSON validation"
        ((WARNINGS++))
    fi
else
    echo -e "${RED}✗${NC} manifest.json not found"
    ((ERRORS++))
fi
echo ""

# Check service worker basics
echo "Checking Service Worker:"
echo "------------------------"
if [ -f "sw.js" ]; then
    if grep -q "CACHE_VERSION" sw.js; then
        echo -e "${GREEN}✓${NC} CACHE_VERSION found in sw.js"
    else
        echo -e "${YELLOW}⚠${NC} CACHE_VERSION not found in sw.js"
        ((WARNINGS++))
    fi

    if grep -q "addEventListener('install'" sw.js; then
        echo -e "${GREEN}✓${NC} Install event handler found"
    else
        echo -e "${RED}✗${NC} Install event handler missing"
        ((ERRORS++))
    fi

    if grep -q "addEventListener('fetch'" sw.js; then
        echo -e "${GREEN}✓${NC} Fetch event handler found"
    else
        echo -e "${RED}✗${NC} Fetch event handler missing"
        ((ERRORS++))
    fi
else
    echo -e "${RED}✗${NC} sw.js not found"
    ((ERRORS++))
fi
echo ""

# Check if icons script is executable
echo "Checking Permissions:"
echo "---------------------"
if [ -f "icons/generate-icons.sh" ]; then
    if [ -x "icons/generate-icons.sh" ]; then
        echo -e "${GREEN}✓${NC} generate-icons.sh is executable"
    else
        echo -e "${YELLOW}⚠${NC} generate-icons.sh is not executable"
        echo "  Run: chmod +x icons/generate-icons.sh"
        ((WARNINGS++))
    fi
fi
echo ""

# Check for icon generation tools
echo "Checking Icon Generation Tools:"
echo "--------------------------------"
TOOL_FOUND=0

if command -v rsvg-convert &> /dev/null; then
    echo -e "${GREEN}✓${NC} rsvg-convert found (librsvg)"
    TOOL_FOUND=1
fi

if command -v convert &> /dev/null; then
    echo -e "${GREEN}✓${NC} convert found (ImageMagick)"
    TOOL_FOUND=1
fi

if command -v inkscape &> /dev/null; then
    echo -e "${GREEN}✓${NC} inkscape found"
    TOOL_FOUND=1
fi

if [ $TOOL_FOUND -eq 0 ]; then
    echo -e "${YELLOW}⚠${NC} No icon generation tools found"
    echo "  Install one of: librsvg, ImageMagick, or Inkscape"
    echo "  macOS: brew install librsvg"
    echo "  Linux: sudo apt install librsvg2-bin"
    ((WARNINGS++))
fi
echo ""

# Summary
echo "================================"
echo "Summary:"
echo "--------"
if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Generate icons: cd icons && ./generate-icons.sh"
    echo "2. Start server: cd .. && python3 -m http.server 8000"
    echo "3. Visit: http://localhost:8000"
    echo "4. Test PWA: http://localhost:8000/pwa-test.html"
elif [ $ERRORS -eq 0 ]; then
    echo -e "${YELLOW}⚠ $WARNINGS warning(s) found${NC}"
    echo ""
    echo "Your PWA setup is functional but has optional items missing."
    echo "Check the warnings above for details."
else
    echo -e "${RED}✗ $ERRORS error(s) and $WARNINGS warning(s) found${NC}"
    echo ""
    echo "Please fix the errors above before proceeding."
    exit 1
fi

echo ""
echo "For more information, see SETUP.md"
