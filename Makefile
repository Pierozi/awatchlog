build:
	cargo build

release:
	cargo build --release

run:
	RUST_BACKTRACE=full RUST_BACKTRACE=1 ./target/debug/awatchlog -c tests/rust/config.toml --credentials tests/rust/credentials.toml
