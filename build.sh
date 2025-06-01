#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Determine the project name from Cargo.toml
PROJECT_NAME=$(grep '^name *=' Cargo.toml | head -n 1 | sed 's/name *= *"\(.*\)"/\1/')

if [ -z "$PROJECT_NAME" ]; then
    echo "Error: Could not determine project name from Cargo.toml"
    exit 1
fi

echo "Starting release build for ${PROJECT_NAME}..."

# Build the project in release mode.
cargo build --release

echo "Build complete!"
echo "Binary located at: target/release/${PROJECT_NAME}"

# Copy the binary to the project root
echo "Copying target/release/${PROJECT_NAME} to ./${PROJECT_NAME}..."
cp "target/release/${PROJECT_NAME}" "./${PROJECT_NAME}"
echo "Binary ${PROJECT_NAME} is now available in the project root." 