# Autofill Parser

A high-performance autofill data parser that processes multiple input files, merges records based on common identifiers, and outputs consolidated data in NDJSON format. Written in Rust for exceptional speed, memory efficiency, and reliability. Capable of processing multi-GB files with configurable memory management.

## Purpose

The primary goal of this tool is to:
1.  Read data from all files within a specified input directory.
2.  Parse each line, which is expected to be a comma-separated list of key:value pairs.
3.  Identify a primary key for each record (preferring emails, then 'identifier' field if it's an email, then 'username', then 'login').
4.  Merge data for the same user from different lines or files. The merging strategy is to keep the first encountered value for any given field (excluding the primary identifier and email list, which are handled specially).
5.  Output each unique user record as a JSON object on a new line (NDJSON format) to a specified output file or directory.
6.  Log processing errors and skipped lines to `processing_errors.log` for review.

## Installation

1.  **Install Rust**: If you don't have Rust installed, follow the official instructions at [rust-lang.org](https://www.rust-lang.org/tools/install).
2.  **Clone the Repository**:
    ```bash
    git clone <repository-url>
    cd autofill-parser
    ```
3.  **Build the Project**:
    You can use the provided `build.sh` script or `make`:
    *   Using `build.sh` (creates `./autofill_parser` executable in the project root):
        ```bash
        chmod +x build.sh
        ./build.sh
        ```
    *   Using `Makefile` (also creates `./autofill_parser`):
        ```bash
        make build 
        ```
    This will compile the project in release mode. The executable will be located at `target/release/autofill_parser` and also copied to `./autofill_parser` in the project root if you use `build.sh` or `make build`.

## How to Run

Once built, you can run the program from the project root directory:

```bash
./autofill_parser --input <INPUT_DIRECTORY_PATH> --output <OUTPUT_FILE_OR_DIRECTORY_PATH>
```

**Arguments**:
*   `-i, --input <INPUT_DIRECTORY_PATH>`: (Required) Path to the input folder containing files to process.
*   `-o, --output <OUTPUT_FILE_OR_DIRECTORY_PATH>`: (Required) Path to the output file or folder. If a folder is specified, output will be saved as `result.ndjson` in that folder.
*   `-t, --threads <NUMBER>`: (Optional) Number of threads for parallel processing (0 = auto-detect, default: 0).
*   `-v, --verbose`: (Optional) Activate verbose mode to print detailed processing information to the console (in addition to `processing_errors.log`).

**Example**:
```bash
./autofill_parser --input ./test_data --output ./test_output/users.ndjson
```
Or, to output to a directory (file will be `results.ndjson` inside it):
```bash
./autofill_parser --input ./test_data --output ./test_output/
```
To run with verbose output:
```bash
./autofill_parser --input ./test_data --output ./test_output/users_verbose.ndjson -v
```
To run with custom thread count:
```bash
./autofill_parser --input ./test_data --output ./test_output/users.ndjson -t 8
```

The `Makefile` also provides a convenience target to run with sample data:
```bash
make run 
```
This will use `test_input/` as input and save results to `test_output/actual_data_result.ndjson`.
There's also `make run-verbose`.

## Performance and Memory Management

This parser is designed for high performance and can handle extremely large datasets:

*   **Speed**: Processes 55,000+ records per second on modern hardware
*   **Memory Safety**: Configurable memory limits prevent OOM crashes
*   **File Size**: Handles files from KB to multi-GB without skipping
*   **Parallelism**: Automatic thread pool sizing based on CPU cores
*   **Adaptive Strategy**: Adjusts processing based on dataset size

The program uses a producer-consumer pattern with memory-aware processing that automatically swaps to disk when approaching memory limits. Configuration can be adjusted in `config.json` for different memory profiles.

## Searching and Formatting the Output

The output file (e.g., `result.ndjson`) is in NDJSON format, meaning each line is a valid JSON object. This makes it easy to process with command-line tools like `ripgrep` (rg) for searching and `jq` for JSON manipulation.

**Prerequisites**:
*   Install `ripgrep` (rg): [Installation Guide](https://github.com/BurntSushi/ripgrep#installation)
*   Install `jq`: [Download jq](https://jqlang.github.io/jq/download/)

**Examples**:

Let's assume your output file is `test_output/actual_data_result.ndjson`.

1.  **Find lines containing a specific email or keyword using `rg`**:
    ```bash
    rg "user@example.com" test_output/actual_data_result.ndjson
    ```
    This will print all lines (JSON objects) that contain "user@example.com".

2.  **Find lines and pretty-print the matching JSON using `rg` and `jq`**:
    ```bash
    rg "some_username" test_output/actual_data_result.ndjson | jq '.'
    ```
    *   `rg "some_username" ...` finds the lines.
    *   `| jq '.'` pipes each matching line to `jq`, and `.'` tells `jq` to pretty-print the entire JSON object.

3.  **Find lines and extract specific fields using `rg` and `jq`**:
    Suppose you want to find users with "some_value" in any of their fields and display only their `identifier` and `emails`:
    ```bash
    rg "some_value" test_output/actual_data_result.ndjson | jq '{identifier: .identifier, emails: .emails}'
    ```
    This will output a new JSON object for each match, containing only the specified fields.

4.  **Search for a specific value in a specific field (more advanced `jq` filtering after `rg`)**:
    If you want to find users where the `username` field is exactly "targetuser":
    ```bash
    # First, rg might narrow down lines that mention "targetuser"
    rg "targetuser" test_output/actual_data_result.ndjson | jq 'select(.other_fields.username == "targetuser")'
    ```
    Or, if "username" might not always be in `other_fields` (e.g., it could be the main identifier):
    ```bash
    rg "targetuser" test_output/actual_data_result.ndjson | jq 'select(.identifier == "targetuser" or .other_fields.username == "targetuser" or .other_fields.login == "targetuser")'
    ```
    These examples show how you can combine the power of `rg` for fast text searching with `jq` for structured JSON querying and transformation.

## License

This project is licensed under the MIT License.

```
MIT License

Copyright (c) 2025

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
``` 