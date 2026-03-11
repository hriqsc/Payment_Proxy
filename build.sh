#backend build

proj_dir=$(pwd)
cd backend
cargo build --release --target x86_64-unknown-linux-musl #static binary
cp -f target/x86_64-unknown-linux-musl/release/backend $proj_dir/build

cd $proj_dir

docker compose up --build --force-recreate