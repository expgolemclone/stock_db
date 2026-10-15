#!/usr/bin/env bash

rust_workspace_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
export CARGO_HOME="$rust_workspace_dir/.cargo"
export PATH="$rust_workspace_dir/.rust-toolchain/bin:$CARGO_HOME/bin:$PATH"
export LD_LIBRARY_PATH="$rust_workspace_dir/.rust-toolchain/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
unset rust_workspace_dir
