#!/usr/bin/env bash

set -euo pipefail
IFS=$'\n\t'

readonly RUST_VERSION="1.97.1"
readonly RUST_COMMIT="8bab26f4f"
readonly CARGO_COMMIT="c980f4866"
readonly DIST_DATE="2026-07-16"
readonly HOST_TARGET="x86_64-unknown-linux-gnu"
readonly DIST_BASE_URL="https://static.rust-lang.org/dist/${DIST_DATE}"

setup_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
readonly setup_dir
readonly toolchain_dir="$setup_dir/.rust-toolchain"
readonly cargo_home="$setup_dir/.cargo"
readonly activation_path="$setup_dir/activate-rust.sh"
readonly cargo_config_path="$cargo_home/config.toml"
readonly gitignore_path="$setup_dir/.gitignore"
readonly toolchain_marker="$toolchain_dir/.managed-by-setup-rust"

install_work_dir=""

fail() {
  printf 'setup-rust: %s\n' "$1" >&2
  exit 1
}

cleanup() {
  if [[ -n "$install_work_dir" && -d "$install_work_dir" ]]; then
    rm -r -- "$install_work_dir"
  fi
}

trap cleanup EXIT

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "required command is missing: $1"
}

ensure_gitignore_line() {
  local line=$1
  grep -Fqx -- "$line" "$gitignore_path" 2>/dev/null || printf '%s\n' "$line" >> "$gitignore_path"
}

write_workspace_files() {
  mkdir -p -- "$cargo_home"

  cat > "$activation_path" <<'ACTIVATE'
#!/usr/bin/env bash

rust_workspace_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
export CARGO_HOME="$rust_workspace_dir/.cargo"
export PATH="$rust_workspace_dir/.rust-toolchain/bin:$CARGO_HOME/bin:$PATH"
export LD_LIBRARY_PATH="$rust_workspace_dir/.rust-toolchain/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
unset rust_workspace_dir
ACTIVATE
  chmod 755 "$activation_path"

  cat > "$cargo_config_path" <<'CARGO_CONFIG'
[build]
rustflags = ["-C", "linker-features=-lld"]
CARGO_CONFIG
  chmod 644 "$cargo_config_path"

  touch "$gitignore_path"
  ensure_gitignore_line '/.cargo/*'
  ensure_gitignore_line '!/.cargo/config.toml'
  ensure_gitignore_line '/.rust-install/'
  ensure_gitignore_line '/.rust-install.*/'
  ensure_gitignore_line '/.rust-toolchain/'
  ensure_gitignore_line '/target/'
}

toolchain_is_ready() {
  local rustc_version
  local cargo_version

  [[ -x "$toolchain_dir/bin/rustc" && -x "$toolchain_dir/bin/cargo" ]] || return 1

  rustc_version=$(LD_LIBRARY_PATH="$toolchain_dir/lib" "$toolchain_dir/bin/rustc" --version 2>/dev/null) || return 1
  cargo_version=$(LD_LIBRARY_PATH="$toolchain_dir/lib" "$toolchain_dir/bin/cargo" --version 2>/dev/null) || return 1

  [[ "$rustc_version" == "rustc ${RUST_VERSION} (${RUST_COMMIT} "* ]] || return 1
  [[ "$cargo_version" == "cargo ${RUST_VERSION} (${CARGO_COMMIT} "* ]] || return 1
}

download_and_verify() {
  local package=$1
  local checksum=$2
  local archive_name="${package}-${RUST_VERSION}-${HOST_TARGET}.tar.xz"
  local archive_path="$install_work_dir/$archive_name"

  printf 'Downloading %s\n' "$archive_name"
  curl --proto '=https' --tlsv1.2 -fL --retry 3 \
    -o "$archive_path" "$DIST_BASE_URL/$archive_name"
  printf '%s  %s\n' "$checksum" "$archive_path" | sha256sum -c -
}

install_component() {
  local package=$1
  local component_dir=$package
  local archive_root="${package}-${RUST_VERSION}-${HOST_TARGET}"
  local -a exclude_args=()

  if [[ "$package" == 'rust-std' ]]; then
    component_dir="rust-std-${HOST_TARGET}"
    exclude_args+=(
      "--exclude=$archive_root/$component_dir/lib/*/librustc-stable_rt.*"
    )
  elif [[ "$package" == 'rustc' ]]; then
    exclude_args+=(
      "--exclude=$archive_root/$component_dir/bin/rustdoc"
      "--exclude=$archive_root/$component_dir/libexec/*"
      "--exclude=$archive_root/$component_dir/share/*"
    )
  fi

  tar -xJf "$install_work_dir/${package}-${RUST_VERSION}-${HOST_TARGET}.tar.xz" \
    -C "$install_work_dir/toolchain" \
    --strip-components=2 \
    "${exclude_args[@]}" \
    "$archive_root/$component_dir"
}

repair_libllvm() {
  local archive_path="$install_work_dir/rustc-${RUST_VERSION}-${HOST_TARGET}.tar.xz"
  local archive_root="rustc-${RUST_VERSION}-${HOST_TARGET}"
  local library_name="libLLVM.so.22.1-rust-${RUST_VERSION}-stable"
  local archive_member="$archive_root/rustc/lib/$library_name"
  local target_path="$install_work_dir/toolchain/lib/$library_name"
  local repair_path="$install_work_dir/$library_name.repaired"
  local expected_size=199494192
  local actual_size

  # Some quota-limited overlay filesystems silently truncate this large member
  # during normal directory extraction. Stream that one member into a fresh file
  # and validate it before replacing the extracted copy.
  tar -xOJf "$archive_path" "$archive_member" > "$repair_path"
  actual_size=$(stat -c '%s' "$repair_path")
  [[ "$actual_size" == "$expected_size" ]] ||
    fail "unexpected $library_name size: $actual_size (expected $expected_size)"
  readelf -h "$repair_path" >/dev/null || fail "$library_name is not a valid ELF file"
  chmod 644 "$repair_path"
  mv -- "$repair_path" "$target_path"
}

install_toolchain() {
  if [[ -e "$toolchain_dir" && ! -f "$toolchain_marker" ]]; then
    fail "refusing to replace unowned directory: $toolchain_dir"
  fi
  if [[ -d "$toolchain_dir" ]]; then
    rm -r -- "$toolchain_dir"
  fi

  install_work_dir=$(mktemp -d "$setup_dir/.rust-install.XXXXXX")
  mkdir -p -- "$install_work_dir/toolchain"

  download_and_verify rustc \
    '9819d0a32d56bd339585319c80260e332779f5541fd66838ab7e016d6c814819'
  install_component rustc
  repair_libllvm
  rm -- "$install_work_dir/rustc-${RUST_VERSION}-${HOST_TARGET}.tar.xz"

  download_and_verify cargo \
    'e1be5f5ff7f7f80ca506fb65770b759edbdc6d303781ed71c5de8ec8a8394779'
  install_component cargo
  rm -- "$install_work_dir/cargo-${RUST_VERSION}-${HOST_TARGET}.tar.xz"

  download_and_verify rust-std \
    '1c1e704ae80126b7de34f72ea2825f7fd01736dec20732faed47374b95282fba'
  install_component rust-std
  rm -- "$install_work_dir/rust-std-${RUST_VERSION}-${HOST_TARGET}.tar.xz"

  printf '%s\n' "$RUST_VERSION" > "$install_work_dir/toolchain/.managed-by-setup-rust"

  mv -- "$install_work_dir/toolchain" "$toolchain_dir"
}

verify_cargo_build() {
  local smoke_dir

  if [[ -z "$install_work_dir" ]]; then
    install_work_dir=$(mktemp -d "$setup_dir/.rust-install.XXXXXX")
  fi
  smoke_dir="$install_work_dir/smoke-test"

  cat > "$install_work_dir/Cargo.toml" <<'SMOKE_WORKSPACE'
[workspace]
members = []
resolver = "3"
SMOKE_WORKSPACE

  CARGO_HOME="$cargo_home" \
  LD_LIBRARY_PATH="$toolchain_dir/lib" \
  PATH="$toolchain_dir/bin:$cargo_home/bin:$PATH" \
    cargo init --quiet --bin --name cargo_setup_smoke "$smoke_dir"

  CARGO_HOME="$cargo_home" \
  CARGO_TARGET_DIR="$smoke_dir/target" \
  LD_LIBRARY_PATH="$toolchain_dir/lib" \
  PATH="$toolchain_dir/bin:$cargo_home/bin:$PATH" \
    cargo build --quiet --manifest-path "$smoke_dir/Cargo.toml"

  "$smoke_dir/target/debug/cargo_setup_smoke" >/dev/null
}

main() {
  [[ $(uname -s) == 'Linux' ]] || fail 'only Linux is supported'
  [[ $(uname -m) == 'x86_64' ]] || fail 'only x86_64 is supported'

  local required_command
  for required_command in bash cc curl grep ld mktemp mv readelf sha256sum stat tar xz; do
    require_command "$required_command"
  done

  write_workspace_files

  if toolchain_is_ready; then
    printf '%s\n' "$RUST_VERSION" > "$toolchain_marker"
    printf 'Rust %s is already installed.\n' "$RUST_VERSION"
  else
    install_toolchain
  fi

  toolchain_is_ready || fail 'installed toolchain validation failed'
  verify_cargo_build

  printf '\nRust %s and Cargo %s are ready.\n' "$RUST_VERSION" "$RUST_VERSION"
  printf 'Run: source %q\n' "$activation_path"
  printf 'Then: cargo build\n'
}

main "$@"
