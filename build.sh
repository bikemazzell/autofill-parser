#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Determine the project name from Cargo.toml
PROJECT_NAME=$(grep '^name *=' Cargo.toml | head -n 1 | sed 's/name *= *"\(.*\)"/\1/')

if [ -z "$PROJECT_NAME" ]; then
    echo "Error: Could not determine project name from Cargo.toml"
    exit 1
fi

# Function to detect CPU count
detect_threads() {
    if command -v nproc >/dev/null 2>&1; then
        nproc
    elif [ -f /proc/cpuinfo ]; then
        grep -c ^processor /proc/cpuinfo
    elif command -v sysctl >/dev/null 2>&1; then
        sysctl -n hw.ncpu 2>/dev/null || echo "4"
    else
        echo "4"  # fallback
    fi
}

# Function to show usage
show_usage() {
    echo "Usage: $0"
    echo ""
    echo "Builds the autofill parser in release mode."
    echo ""
    echo "Examples:"
    echo "  ./build.sh"
    echo "  ./build.sh help"
}

# Parse arguments
MODE=${1:-build}

case "$MODE" in
    "build"|"")
        echo "Building release version..."
        cargo build --release
        cp "target/release/${PROJECT_NAME}" "./${PROJECT_NAME}"
        echo "✅ Binary: ./${PROJECT_NAME}"
        
        THREADS=$(detect_threads)
        echo "Usage: ./${PROJECT_NAME} -i input_dir -o output.ndjson -t ${THREADS}"
        ;;
    
    "help"|"-h"|"--help")
        show_usage
        exit 0
        ;;
    
    *)
        echo "❌ Unknown option: $MODE"
        echo ""
        show_usage
        exit 1
        ;;
esac

echo ""
echo "🚀 Build complete!"