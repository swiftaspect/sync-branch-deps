# Multi-stage, multi-arch build: compile a static (musl) `sbd` binary for the
# build container's native arch, then ship it on distroless static — a ~2 MB
# base with CA certs and a nonroot user, ideal for a static binary. Under a
# `--platform linux/amd64,linux/arm64` build each arch compiles natively (via
# emulation), so no cross-toolchain is needed.

FROM docker.io/library/rust:1@sha256:1f0dbad1df66647807e6952d1db85d0b2bda7606cb2139d82517e4f009967376 AS builder
# ring (via rustls/ureq) needs a C toolchain for the musl target; musl-gcc is
# the native musl compiler in whichever arch the build runs as.
ENV CC_x86_64_unknown_linux_musl=musl-gcc \
    CC_aarch64_unknown_linux_musl=musl-gcc
WORKDIR /src
RUN apt-get update \
 && apt-get install -y --no-install-recommends musl-tools \
 && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN set -eux; \
    case "$(uname -m)" in \
      x86_64)  TRIPLE=x86_64-unknown-linux-musl ;; \
      aarch64) TRIPLE=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported arch: $(uname -m)" >&2; exit 1 ;; \
    esac; \
    rustup target add "$TRIPLE"; \
    cargo build --release --target "$TRIPLE"; \
    install -Dm755 "target/$TRIPLE/release/sbd" /out/sbd

FROM gcr.io/distroless/static-debian12:nonroot@sha256:aef9602f8710ec12bde19d593fed1f76c708531bb7aba205110f1029786ead7b
COPY --from=builder /out/sbd /usr/local/bin/sbd
ENTRYPOINT ["/usr/local/bin/sbd"]

LABEL org.opencontainers.image.title="sync-branch-deps" \
      org.opencontainers.image.description="Resolve cross-repo feature-branch dependencies to matching pre-release artifacts." \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/swiftaspect/sync-branch-deps"
