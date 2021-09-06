CHANNEL_BUILDER_DEPS := src/*.rs migrations/*/*.sql Cargo.toml

debug: target/debug/channel_builder
release: target/release/channel_builder

target/debug/channel_builder: ${CHANNEL_BUILDER_DEPS}
	cargo build

target/release/channel_builder: ${CHANNEL_BUILDER_DEPS}
	cargo build --release


.PHONY: debug release
