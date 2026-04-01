.PHONY: p pretty l lint t ta test-all b build r run br build-release c check

p pretty:
	cargo fmt --all

l lint:
	cargo clippy --all-targets --all-features -- -D warnings

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
