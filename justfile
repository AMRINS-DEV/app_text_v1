set shell := ["bash", "-cu"]

# Bring up all datastores + observability (NATS, QuestDB, Postgres+Timescale, FalkorDB, Qdrant, DragonflyDB, Grafana, Prometheus, Loki)
infra-up:
    docker compose -f infra/docker/compose.yml up -d

infra-down:
    docker compose -f infra/docker/compose.yml down

infra-logs:
    docker compose -f infra/docker/compose.yml logs -f

# Run every service in dev mode (requires infra-up first)
dev:
    just infra-up
    (cd services/agents && uv run --all-packages python -c "print('agents layer: Phase 0 scaffold, no long-running dev process yet')" &)
    pnpm --filter @tradeos/gateway dev &
    pnpm --filter @tradeos/dashboard dev &
    cargo run -p tradeos-core

# Build everything: Rust workspace + TS workspace + Python packages
build:
    cargo build --workspace
    pnpm -r build
    cd services/agents && uv sync --all-packages

# Test everything
test:
    cargo nextest run --workspace || cargo test --workspace
    pnpm -r test
    cd services/agents && uv run --all-packages pytest

# Rust/criterion benchmarks (CI fails on >10% regression, see .github/workflows/ci.yml)
bench:
    cargo bench --workspace

# Lint everything
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    pnpm -r lint
    cd services/agents && uv run ruff check .
    cd services/agents && uv run mypy packages/*/src

# Regenerate protobuf clients for Rust/TS/Python from packages/proto/*.proto
proto:
    bash packages/proto/generate.sh

# Deterministic replay regression harness (bit-identical order stream from recorded ticks)
replay file:
    cargo run -p tradeos-replay -- --input {{file}}

# Run the MT5 bridge test double standing in for a real MQL5 terminal (§5.4)
mock-bridge:
    cargo run -p mock-mt5-bridge -- --tick-addr tcp://127.0.0.1:28001 --order-addr tcp://127.0.0.1:28002
