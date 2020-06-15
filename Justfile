# wengwengweng

name := "fopen"
version := "0.0.0"

check:
	cargo c \
		--all-features

run +args="":
	cargo run --release -- {{args}}

build:
	cargo build --release

macos: build
	rm -rf dist/{{name}}.app
	rm -rf dist/{{name}}_v{{version}}_mac.tar.gz
	upx target/release/{{name}} -o {{name}}
	packapp {{name}} --name {{name}} -o dist/{{name}}.app
	cd dist; \
		zip -r -9 {{name}}_v{{version}}_mac.zip {{name}}.app
	rm {{name}}

web:
	cargo build \
		--release \
		--target wasm32-unknown-unknown
	wasm-bindgen target/wasm32-unknown-unknown/release/{{name}}.wasm \
		--out-dir site \
		--target web \
		--no-typescript

bake:
	cargo run --bin bake --release

doc crate:
	cargo doc \
		--no-deps \
		--open \
		-p {{crate}}

bloat:
	cargo bloat --release --crates

update:
	cargo update
	cargo outdated --root-deps-only

loc:
	loc

