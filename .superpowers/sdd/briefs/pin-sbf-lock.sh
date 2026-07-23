#!/usr/bin/env bash
set -euo pipefail
cd /workspace
cargo generate-lockfile
cargo update -p base64ct --precise 1.6.0 || true
cargo update -p clap_lex --precise 0.7.4 || true
cargo update -p indexmap --precise 2.6.0 || true
cargo update -p crypto-common --precise 0.1.6 || true
cargo update -p block-buffer --precise 0.10.4 || true
for i in $(seq 1 25); do
  if cargo metadata --format-version 1 >/dev/null 2>meta.err; then
    echo "METADATA_OK after $i attempts"
    exit 0
  fi
  line=$(grep "failed to parse manifest" meta.err | head -1 || true)
  echo "Attempt $i: $line"
  crate=$(echo "$line" | sed -n 's|.*/\([a-zA-Z0-9_-]*\)-[0-9].*/Cargo.toml|\1|p')
  case "$crate" in
    base64ct) cargo update -p base64ct --precise 1.6.0 ;;
    clap_lex) cargo update -p clap_lex --precise 0.7.4 ;;
    indexmap) cargo update -p indexmap --precise 2.6.0 ;;
    crypto-common) cargo update -p crypto-common --precise 0.1.6 ;;
    block-buffer) cargo update -p block-buffer --precise 0.10.4 ;;
    getrandom) cargo update -p getrandom --precise 0.2.15 ;;
    cc) cargo update -p cc --precise 1.1.30 ;;
    home) cargo update -p home --precise 0.5.9 ;;
    log) cargo update -p log --precise 0.4.22 ;;
    generic-array) cargo update -p generic-array --precise 0.14.7 ;;
    *)
      echo "No pin for $crate"; cat meta.err; exit 1 ;;
  esac
done
echo "Gave up"; cat meta.err; exit 1
