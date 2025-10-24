# Simple convenience targets for LoadLynx

.PHONY: all g431-build g431-run s3-build fmt

all: g431-build

g431-build:
	cd firmware/analog && cargo build

g431-run:
	cd firmware/analog && cargo run

s3-build:
	cd firmware/digital && cargo +esp build

fmt:
	cargo fmt --all || true
