#!/usr/bin/env bash
# Regenerates TS (ts-proto) and Python (grpcio-tools) clients from packages/proto/*.proto.
# Rust codegen happens at build time via crates/domain/build.rs (prost-build), not here.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

command -v protoc >/dev/null || { echo "protoc not found (apt install protobuf-compiler)"; exit 1; }

PROTOS=(common.proto tick.proto signal.proto order.proto)

# --- TypeScript (ts-proto) ---
mkdir -p gen/ts
protoc \
  --plugin=protoc-gen-ts_proto="$(dirname "$0")/../../node_modules/.bin/protoc-gen-ts_proto" \
  --ts_proto_out=gen/ts \
  --ts_proto_opt=esModuleInterop=true,outputJsonMethods=false,useOptionals=messages \
  "${PROTOS[@]}"

# --- Python (grpcio-tools) ---
mkdir -p gen/python
python3 -m grpc_tools.protoc \
  -I. \
  --python_out=gen/python \
  --pyi_out=gen/python \
  "${PROTOS[@]}"

echo "Generated TS -> packages/proto/gen/ts, Python -> packages/proto/gen/python"
echo "Rust bindings regenerate automatically on 'cargo build' via crates/domain/build.rs"
