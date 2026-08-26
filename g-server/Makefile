### Prerequisite:
# - cargo-llvm-cov: cargo install cargo-llvm-cov

## Using cargo-llvm-cov to generate lcov.info and html coverage reports covering functions, lines, and regions coverages.
# `branch` uses nightly because cargo-llvm-cov --branch is unstable.

OUT_DIR := target/coverage
OUT_FILE := $(OUT_DIR)/lcov.info
MIN_COVERAGE ?= 95

check:
	## enable if there are divergent feature switch. 
	# @RUSTFLAGS="-D warnings" cargo check
	# @cargo clippy -- -D warnings
	# @RUSTDOCFLAGS="-D warnings" cargo doc
	# all features enabled
	@RUSTFLAGS="-D warnings" cargo check --all-features
	@cargo clippy --all-features -- -D warnings
	@RUSTDOCFLAGS="-D warnings" cargo doc --all-features

test:
	@echo "Running tests..."
	@cargo test --all-features
	@cargo test -q --doc --all-features

lcov:
	@echo "Generating lcov.info..."
	@mkdir -p $(OUT_DIR)
	@cargo llvm-cov test --all-features \
		--output-path $(OUT_FILE) \
		--lcov \
		--fail-under-lines $(MIN_COVERAGE) \
		--ignore-filename-regex \
			"_test\.rs$$|\
			tests/|\
			examples/"

html:
	@echo "Generating html coverage report..."
	@mkdir -p $(OUT_DIR)
	@cargo llvm-cov test --all-features \
		--output-dir $(OUT_DIR) \
		--open \
		--ignore-filename-regex \
			"_test\.rs$$|\
			tests/|\
			examples/"

branch:
	@echo "Generate coverage with branch coverage..."
	@mkdir -p $(OUT_DIR)
	@cargo +nightly llvm-cov test --all-features \
		--output-dir $(OUT_DIR) \
		--open \
		--branch \
		--ignore-filename-regex \
			"_test\.rs$$|\
			tests/|\
			examples/"

all:
	@echo "Running all checks..."
	@echo "Running cargo check---------------------------------------------"
	# @cargo check
	@cargo check --all-features
	@sleep 1
	@echo "Running formatting----------------------------------------------"
	@cargo fmt --all
	@sleep 1
	@echo "Running clippy--------------------------------------------------"
	@cargo clippy --all-features -- -D warnings
	@sleep 1
	@echo "Running doc"
	@cargo doc --all-features --no-deps
	@sleep 1
	@echo "Running tests---------------------------------------------------"
	@cargo test -q --all-features
	@cargo test -q --doc --all-features


# --------------------------------------------------------------------------------
# -----------------------------------BENCHMARKS-----------------------------------
# --------------------------------------------------------------------------------
BENCH_DIR := target/criterion
BASELINE_DIR := benchmarks

.PHONY: bench-save
bench-save:
	@test -n "$(NAME)" || (echo "Usage: make bench-save NAME=<baseline>"; exit 1)
	cargo bench --bench bench -- --save-baseline $(NAME)
	mkdir -p $(BASELINE_DIR)
	cp -a $(BENCH_DIR)/. $(BASELINE_DIR)/

.PHONY: bench-restore
bench-restore:
	@test -n "$(NAME)" || (echo "Usage: make bench-restore NAME=<baseline>"; exit 1)
	mkdir -p $(BENCH_DIR)
	cp -a $(BASELINE_DIR)/. $(BENCH_DIR)/

.PHONY: bench-compare
bench-compare:
	@test -n "$(NAME)" || (echo "Usage: make bench-compare NAME=<baseline>"; exit 1)
	$(MAKE) bench-restore
	cargo bench --bench bench -- --baseline $(NAME)

.PHONY: bench-clean
bench-clean:
	rm -rf $(BENCH_DIR)
