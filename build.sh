#!/bin/sh

cargo build --target wasm32-unknown-unknown --release --package $1
WASM_FILE=target/wasm32-unknown-unknown/release/$1.wasm
ic-wasm $WASM_FILE -o $WASM_FILE shrink
gzip -nf9v target/wasm32-unknown-unknown/release/$1.wasm
