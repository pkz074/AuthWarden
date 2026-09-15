.PHONY: test audit coverage coverage-all

test:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test

audit:
	cargo audit --ignore RUSTSEC-2023-0071

coverage:
	rustup run stable cargo llvm-cov --all-targets --summary-only

coverage-all:
	rustup run stable cargo llvm-cov --all-targets --summary-only -- --include-ignored
