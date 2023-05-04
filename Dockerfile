FROM rust:alpine3.17 AS build

WORKDIR /risc0_template

COPY ./ ./

RUN apk add --no-cache g++

RUN apk add --no-cache musl-dev

RUN cargo build --release

FROM alpine:3.17

COPY --from=build /risc0_template/target/release/risc0_template /init