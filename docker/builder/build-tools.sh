#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=cli
# TODO use an unstripped cli profile, we dont need to use release image for
# forge cli, that might save build time / space without losing backtrace
FORGE_PROFILE=release

echo "Building tools and services docker images"
echo "PROFILE: $PROFILE"
echo "FORGE_PROFILE: $PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

cargo build --locked --profile=$FORGE_PROFILE \
    -p aptos-forge-cli \
    "$@" &

pid=$!

trap "echo 'ensuring cargo is stopped' && ps $pid && kill $pid" EXIT

echo "Forge running as $pid"

# Build all the rust binaries
cargo build --locked --profile=$PROFILE \
    -p aptos \
    -p aptos-backup-cli \
    -p aptos-faucet-service \
    -p aptos-fn-check-client \
    -p aptos-node-checker \
    -p aptos-openapi-spec-generator \
    -p aptos-telemetry-service \
    -p aptos-debugger \
    -p aptos-transaction-emitter \
    -p aptos-indexer-grpc-cache-worker \
    -p aptos-indexer-grpc-file-store \
    -p aptos-indexer-grpc-data-service \
    -p aptos-indexer-grpc-post-processor \
    -p aptos-nft-metadata-crawler-parser \
    -p aptos-api-tester \
    "$@"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos
    aptos-faucet-service
    aptos-node-checker
    aptos-openapi-spec-generator
    aptos-telemetry-service
    aptos-fn-check-client
    aptos-debugger
    aptos-transaction-emitter
    aptos-indexer-grpc-cache-worker
    aptos-indexer-grpc-file-store
    aptos-indexer-grpc-data-service
    aptos-indexer-grpc-post-processor
    aptos-nft-metadata-crawler-parser
    aptos-api-tester
)

mkdir dist

for BIN in "${BINS[@]}"; do
    cp $CARGO_TARGET_DIR/$PROFILE/$BIN dist/$BIN
done

echo "Waiting for forge build to finish"
wait $pid

# Explicitly copy forge because it uses a different profile
cp $CARGO_TARGET_DIR/$FORGE_PROFILE/forge dist/forge

# Build the Aptos Move framework and place it in dist. It can be found afterwards in the current directory.
echo "Building the Aptos Move framework..."
(cd dist && cargo run --locked --profile=$PROFILE --package aptos-framework -- release)
