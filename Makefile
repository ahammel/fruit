CARGO_MUTANTS_VERSION := $(shell cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; print(json.load(sys.stdin)['metadata']['tools']['cargo-mutants'])")
CARGO_MUTANTS_URL := https://github.com/sourcefrog/cargo-mutants/releases/download/v$(CARGO_MUTANTS_VERSION)/cargo-mutants-x86_64-apple-darwin.tar.gz

MUTANTS_FILES = mutants.out mutants.old.out

.PHONY: p pretty pc pretty-check l lint t ta test-all b build r run br build-release c check tc test-coverage tcr test-coverage-report tm test-mutation w watch clean

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
		--ignore-filename-regex="(command_line_service|_tests\.rs)" \
		--fail-under-lines 100 \
		--fail-under-functions 100

tcr test-coverage-report:
	rustup run stable cargo llvm-cov --all \
		--ignore-filename-regex="(command_line_service|_tests\.rs)" \
		--html --open

bin/cargo-mutants:
	mkdir -p bin
	curl -fsSL "$(CARGO_MUTANTS_URL)" -o /tmp/cargo-mutants.tar.gz
	tar -xzf /tmp/cargo-mutants.tar.gz -C bin/ cargo-mutants
	rm /tmp/cargo-mutants.tar.gz

tm test-mutation: bin/cargo-mutants
	PATH="$(CURDIR)/bin:$$PATH" cargo mutants \
		--exclude "command_line_service/**" \
		-j 4

w watch:
	fd .rs | entr -s 'clear && make c && make l && make tc && make pc'

clean:
	cargo clean
	rm -rf $(MUTANTS_FILES)
	rm -rf bin/
