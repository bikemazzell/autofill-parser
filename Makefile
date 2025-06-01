# Attempt to get the project name from Cargo.toml
PROJECT_NAME := $(shell sed -n 's/^name = "\\(.*\\)"/\\1/p' Cargo.toml | head -n 1)

# If PROJECT_NAME is empty (e.g. sed failed or Cargo.toml is unusual), default it.
ifeq ($(PROJECT_NAME),)
    PROJECT_NAME := autofill_parser
endif

TARGET_DIR := target
RELEASE_DIR := $(TARGET_DIR)/release
SOURCE_BINARY := $(RELEASE_DIR)/$(PROJECT_NAME)
DEST_BINARY := $(PROJECT_NAME)

# Default target
.PHONY: all
all: build

# Build the release binary and copy it to the root directory
.PHONY: build
build:
	@echo "Building $(PROJECT_NAME) in release mode..."
	@cargo build --release
	@echo "Copying binary from $(SOURCE_BINARY) to $(DEST_BINARY)..."
	@cp $(SOURCE_BINARY) $(DEST_BINARY)
	@echo "Build successful. Binary is at ./$(DEST_BINARY)"

# Run the application with sample arguments (ensure test_data exists)
.PHONY: run
run: build
	@echo "Running ./$(DEST_BINARY) with arguments for test_input folder..."
	./$(DEST_BINARY) --input test_input --output test_output/actual_data_result.ndjson

# Run the application with verbose output
.PHONY: run-verbose
run-verbose: build
	@echo "Running ./$(DEST_BINARY) with arguments for test_input folder (VERBOSE)..."
	./$(DEST_BINARY) --input test_input --output test_output/actual_data_result_verbose.ndjson -v

# Run tests
.PHONY: test
test:
	@echo "Running tests..."
	@cargo test

# Clean the build artifacts and the copied binary
.PHONY: clean
clean:
	@echo "Cleaning up build artifacts and copied binary..."
	@cargo clean
	@rm -f $(DEST_BINARY)
	@echo "Cleanup complete." 