SRCS = $(wildcard domain/src/*.rs)

.PHONY: p pretty pc pretty-check l lint t ta test-all b build r run br build-release c check w watch

p pretty:
	cargo fmt --all

pc pretty-check:
	cargo fmt --check

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

w watch:
	fd .rs | entr -s 'clear && make c && make pc && make l && make t'
