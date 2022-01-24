FROM rust:1.57 as build

# create a new empty shell project
RUN USER=root cargo new busyboredom-website --name busyboredom
WORKDIR /busyboredom-website

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# copy wasm
COPY ./wasm ./wasm

# copy static
COPY ./static ./static

# copy secrets
COPY ./secrets ./secrets

# build for release
RUN rm ./target/release/deps/busyboredom*
RUN cargo build --release

# our final base
FROM debian:bullseye-slim

# copy the build artifact from the build stage
COPY --from=build /busyboredom-website/target/release/busyboredom .

# set the startup command to run your binary
CMD ["./busyboredom"]
