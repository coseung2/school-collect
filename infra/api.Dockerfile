# Build image for the server processes: api, migrator, worker.
#
# Only those three packages are built. The Tauri desktop crate is a workspace
# member, so its sources must exist for Cargo to resolve the workspace, but it is
# never compiled here and no webkit system library is required.
FROM rust:1.98-bookworm AS build

WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY services ./services
COPY migrations ./migrations
COPY apps/app/src-tauri ./apps/app/src-tauri

RUN cargo build --release --locked \
      -p school-collect-api \
      -p school-collect-migrator \
      -p school-collect-worker

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --no-install-recommends -y ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 schoolcollect

COPY --from=build /src/target/release/school-collect-api /usr/local/bin/school-collect-api
COPY --from=build /src/target/release/school-collect-migrator /usr/local/bin/school-collect-migrator
COPY --from=build /src/target/release/school-collect-worker /usr/local/bin/school-collect-worker

USER schoolcollect
EXPOSE 3000

# The platform probes HTTP directly: /health for liveness and /ready for
# readiness, which also verifies that the v2 schema has been migrated.
ENTRYPOINT ["/usr/local/bin/school-collect-api"]
