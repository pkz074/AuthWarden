.PHONY: test coverage coverage-all

test:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test

coverage:
	rustup run stable cargo llvm-cov --all-targets --summary-only

coverage-all:
	rustup run stable cargo llvm-cov --all-targets --summary-only -- --include-ignored
