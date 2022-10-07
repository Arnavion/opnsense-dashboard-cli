.PHONY: build clean default install test

default: build

clean:
	rm -rf Cargo.lock target/

build:
	cargo build --release

install: build
	cp -f target/release/opnsense-dashboard ~/.local/bin/

test:
	cargo test --all
	cargo clippy --all --tests --examples
