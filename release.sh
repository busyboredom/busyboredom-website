#!/bin/bash

cd wasm/
wasm-pack build --release --target web

cd ..
cargo run --release
