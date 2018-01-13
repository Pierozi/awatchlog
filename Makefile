build:
	cargo build

release:
	cargo build --release

run:
	./target/debug/awatchlog -c tests/rust/config.toml --credentials tests/rust/credentials.toml
