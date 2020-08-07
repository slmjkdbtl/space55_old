# wengwengweng

name := "space55"
version := "0.0.0"

check:
	cargo c \
		--all-features

run +args="":
	cargo run --release -- {{args}}

build:
	cargo build --release

macos: build
	sips -s format icns icon.png --out icon.icns
	rm -rf dist/{{name}}.app
	rm -rf dist/{{name}}_v{{version}}_mac.tar.gz
	upx target/release/{{name}} -o {{name}}
	packapp {{name}} \
		--name {{name}} \
		--icon icon.icns \
		--high-res \
		--output dist/{{name}}.app
	cd dist; \
		zip -r -9 {{name}}_v{{version}}_mac.zip {{name}}.app
	rm {{name}}
	rm icon.icns

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

