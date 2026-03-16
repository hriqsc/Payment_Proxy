#!/bin/bash
set -e

docker compose down --volumes --remove-orphans

proj_dir=$(pwd)
cd backend
cargo build --release --target x86_64-unknown-linux-musl #static binary
cp -f target/x86_64-unknown-linux-musl/release/backend $proj_dir/build

cd $proj_dir

docker compose up --build --force-recreate