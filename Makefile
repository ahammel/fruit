SRCS = $(wildcard domain/src/*.rs)
COVERAGE_OUTPUT = target/lcov.info

.PHONY: p pretty pc pretty-check l lint t ta test-all b build r run br build-release c check tc test-coverage w watch

p pretty:
	cargo fmt --all

pc pretty-check:
	cargo fmt --check

l lint:
	cargo clippy --all-targets --all-features -- -D warnings

lf lint-fix:
	cargo clippy --all-targets --all-features --fix -- -D warnings

lff lint-fix-force:
	cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings

t ta test-all:
	cargo test --all

c check:
	cargo check --all

b build:
	cargo build --all

r run:
	cargo run

br build-release:
	cargo build --release --all

tc test-coverage:
	rustup run stable cargo llvm-cov --all \
		--ignore-filename-regex="command_line_service" \
		--fail-under-lines 100 \
		--fail-under-regions 100

tcr test-coverage-report: $(COVERAGE_OUTPUT)
$(COVERAGE_OUTPUT): $(SRC)
	rustup run stable cargo llvm-cov --all \
		--ignore-filename-regex="command_line_service" \
		--fail-under-lines 100 \
		--fail-under-regions 100 \
		--lcov \
		--output-path $(COVERAGE_OUTPUT)

w watch:
	fd .rs | entr -s 'clear && make c && make pc && make l && make t && make tc'
