#!/bin/bash

cd wasm/
wasm-pack build --target web

cd ..
cargo run
