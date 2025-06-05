# Attempt to get the project name from Cargo.toml
PROJECT_NAME := $(shell sed -n 's/^name = "\\(.*\\)"/\\1/p' Cargo.toml | head -n 1)

# If PROJECT_NAME is empty (e.g. sed failed or Cargo.toml is unusual), default it.
ifeq ($(PROJECT_NAME),)
    PROJECT_NAME := autofill_parser
endif

# Detect CPU count for thread recommendations
THREADS := $(shell nproc 2>/dev/null || grep -c ^processor /proc/cpuinfo 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

TARGET_DIR := target
RELEASE_DIR := $(TARGET_DIR)/release
SOURCE_BINARY := $(RELEASE_DIR)/$(PROJECT_NAME)
DEST_BINARY := $(PROJECT_NAME)

# Default target - build release version
.PHONY: all
all: build

# Build the release binary
.PHONY: build
build:
	@echo "Building $(PROJECT_NAME) in release mode..."
	@cargo build --release
	@echo "Copying binary from $(SOURCE_BINARY) to $(DEST_BINARY)..."
	@cp $(SOURCE_BINARY) $(DEST_BINARY)
	@echo "✅ Binary: ./$(DEST_BINARY)"
	@echo "Usage: ./$(DEST_BINARY) -i input_dir -o output.ndjson -t $(THREADS)"

# Run the application
.PHONY: run
run: build
	@echo "Running with test data..."
	./$(DEST_BINARY) --input test --output test_output/result.ndjson --threads $(THREADS)

# Run with verbose output
.PHONY: run-verbose
run-verbose: build
	@echo "Running with verbose output..."
	./$(DEST_BINARY) --input test --output test_output/result_verbose.ndjson --threads $(THREADS) -v

# Run tests
.PHONY: test
test:
	@echo "Running tests..."
	@cargo test

# Show help
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  build        - Build release version (default)"
	@echo "  run          - Run with test data"
	@echo "  run-verbose  - Run with verbose output"
	@echo "  test         - Run tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  help         - Show this help"
	@echo ""
	@echo "Detected CPU cores: $(THREADS)"

# Clean the build artifacts and binaries
.PHONY: clean
clean:
	@echo "Cleaning up build artifacts and binaries..."
	@cargo clean
	@rm -f $(DEST_BINARY)
	@rm -rf test_output
	@echo "Cleanup complete."