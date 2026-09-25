#!/usr/bin/env bash
# Run the production Lance namespace suite on native Linux arm64 from macOS.
set -euo pipefail

cd "$(dirname "$0")/.."
mode=${1:-run}
if [[ "$mode" != compile && "$mode" != run ]]; then
  printf 'usage: %s [compile|run]\n' "$0" >&2
  exit 2
fi

container=toolu159-lance-arm
if ! docker container inspect "$container" >/dev/null 2>&1; then
  docker run --platform linux/arm64 -d --name "$container" ubuntu:24.04 sleep infinity >/dev/null
fi
if [[ $(docker inspect "$container" --format '{{.State.Running}}') != true ]]; then
  docker start "$container" >/dev/null
fi
if [[ $(docker exec "$container" dpkg --print-architecture) != arm64 ]]; then
  printf 'Lance smoke container is not Linux arm64\n' >&2
  exit 1
fi

if ! docker exec "$container" test -x /root/.cargo/bin/cargo-nextest; then
  docker exec "$container" bash -lc '
    set -euo pipefail
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq
    apt-get install -qq -y --no-install-recommends ca-certificates curl git build-essential cmake ninja-build clang libclang-dev pkg-config libssl-dev libdigest-sha-perl gzip jq >/tmp/toolu159-apt.log 2>&1
    curl --fail --location --silent --show-error https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.94.1
    export PATH=/root/.cargo/bin:$PATH
    rustup component add clippy rustfmt
    curl --fail --location --silent --show-error https://get.nexte.st/latest/linux-arm | tar zxf - -C /root/.cargo/bin
  '
fi

archive=$(mktemp /tmp/toolu159-source.XXXXXX.tar.gz)
trap 'rm -f "$archive"' EXIT
COPYFILE_DISABLE=1 tar --no-xattrs -czf "$archive" --exclude='./target' --exclude='./.git' .
docker cp "$archive" "$container:/tmp/toolu159-source.tar.gz"
docker exec "$container" bash -lc '
  set -euo pipefail
  rm -rf /tmp/work
  mkdir -p /tmp/work
  tar -xzf /tmp/toolu159-source.tar.gz -C /tmp/work
'

expected=9592a76d4b24bc1cdd801afe436bc76998f6b99184162153edbb77431f2b5e56
if ! docker exec "$container" test -f /tmp/lance.duckdb_extension.gz; then
  docker exec "$container" curl --fail --location --silent --show-error \
    --connect-timeout 10 --max-time 180 \
    https://extensions.duckdb.org/v1.5.5/linux_arm64/lance.duckdb_extension.gz \
    -o /tmp/lance.duckdb_extension.gz
fi
actual=$(docker exec "$container" shasum -a 256 /tmp/lance.duckdb_extension.gz)
actual=${actual%% *}
if [[ "$actual" != "$expected" ]]; then
  printf 'Lance arm64 extension checksum mismatch: expected %s, found %s\n' \
    "$expected" "$actual" >&2
  exit 1
fi
if ! docker exec "$container" test -f /tmp/lance.duckdb_extension; then
  docker exec "$container" bash -lc \
    'gzip -dc /tmp/lance.duckdb_extension.gz > /tmp/lance.duckdb_extension'
fi

docker exec -w /tmp/work -e CARGO_TARGET_DIR=/tmp/toolu159-target \
  -e CARGO_BUILD_JOBS=1 "$container" bash -lc '
    set -euo pipefail
    export PATH=/root/.cargo/bin:$PATH
    cargo fmt -p toolu-orm-connection -- --check
    cargo clippy -p toolu-orm-connection --no-default-features --features lancedb --all-targets -- -D warnings
    cargo nextest list -p toolu-orm-connection --no-default-features --features lancedb -E "binary(lancedb_namespace_test)"
  '

if [[ "$mode" == run ]]; then
  docker exec -w /tmp/work -e CARGO_TARGET_DIR=/tmp/toolu159-target \
    -e CARGO_BUILD_JOBS=1 -e LANCE_EXTENSION_PATH=/tmp/lance.duckdb_extension \
    "$container" bash -lc '
      set -euo pipefail
      export PATH=/root/.cargo/bin:$PATH
      cargo nextest run -p toolu-orm-connection --no-default-features --features lancedb \
        -E "binary(lancedb_namespace_test) or binary(lancedb_startup_test)" \
        --success-output immediate
    '
fi
