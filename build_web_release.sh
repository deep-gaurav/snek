set -e

curl https://sh.rustup.rs -sSf | sh -s - --default-toolchain stable -y
source ~/.cargo/env

curl -L https://github.com/thedodd/trunk/releases/download/latest/trunk-x86_64-unknown-linux-gnu.tar.gz --output trunk.tar.gz
tar -zxvf trunk.tar.gz

export PATH="$PATH:$PWD"

# cargo install wasm-bindgen-cli

rustup target add wasm32-unknown-unknown

trunk build --release