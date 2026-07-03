.POSIX:

# sync-branch-deps — container-first Rust Makefile.
#
# Every cargo invocation runs inside a pinned Rust container, so the only tool
# you need locally is a container engine (podman or docker). CI uses the same
# targets; it just overrides CONTAINER_ENGINE via the file named below.

## Container coordinates (CONTAINER_REGISTRY/ORG/REPO) live in the committed
## .config.include.mk. Override any ?= variable locally in local.include.mk;
## CI passes its own override file by setting the CI_MAKE_OVERRIDES env var
## (unset locally, so local runs use the podman default).
-include .config.include.mk
-include local.include.mk
-include $(CI_MAKE_OVERRIDES)

CONTAINER_ENGINE ?= podman
RUST_IMAGE ?= docker.io/library/rust:1
CONTAINER_WORK_DIR ?= /work

# Named volumes keep the cargo registry + build cache off the host tree and warm
# across runs. The target volume shadows ./target inside the container.
CARGO_CACHE_VOLUME ?= sync-branch-deps-cargo
TARGET_CACHE_VOLUME ?= sync-branch-deps-target

BASE_COMMAND = $(CONTAINER_ENGINE) run --rm \
	-v "$(CURDIR)":$(CONTAINER_WORK_DIR):Z \
	-v $(CARGO_CACHE_VOLUME):/usr/local/cargo/registry \
	-v $(TARGET_CACHE_VOLUME):$(CONTAINER_WORK_DIR)/target \
	-w $(CONTAINER_WORK_DIR) \
	$(RUST_IMAGE)

CARGO = $(BASE_COMMAND) cargo

# rustfmt + clippy aren't in the base image; add them (no-op once present).
ENSURE_COMPONENTS = rustup component add rustfmt clippy >/dev/null 2>&1

.PHONY: help
help: ## Show this help
	@grep -hE '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-10s\033[0m %s\n", $$1, $$2}'

.PHONY: setup
setup: ## Fetch dependencies and toolchain components
	$(BASE_COMMAND) sh -c "$(ENSURE_COMPONENTS); cargo fetch"

.PHONY: check
check: ## Aggregate gate: format check + clippy + tests (CI's entry point)
	$(BASE_COMMAND) sh -c "$(ENSURE_COMPONENTS) \
		&& cargo fmt --check \
		&& cargo clippy --all-targets -- -D warnings \
		&& cargo test"

.PHONY: fmt
fmt: ## Format the code in place
	$(BASE_COMMAND) sh -c "$(ENSURE_COMPONENTS); cargo fmt"

.PHONY: lint
lint: ## Run clippy with warnings denied
	$(BASE_COMMAND) sh -c "$(ENSURE_COMPONENTS) && cargo clippy --all-targets -- -D warnings"

.PHONY: test
test: ## Run the test suite
	$(CARGO) test

.PHONY: build
build: ## Build the release binary (target/release/sbd)
	$(CARGO) build --release

# --- Release / publish -------------------------------------------------------
# Distribution is a single binary attached to a GitHub Release. `publish` is the
# entry point the release workflow calls on a tag. `publish-crate` is opt-in for
# `cargo install` support (needs a crates.io token + an available crate name).

DIST_DIR ?= dist
# Target triple for the published binary. `dist` builds for this triple
# explicitly and stages the result as sbd-<triple>. Default is the image's
# native glibc target; override to e.g. x86_64-unknown-linux-musl for a fully
# static build — the target's std is added automatically, but musl also needs
# musl-tools (musl-gcc) present in the image.
BINARY_TARGET ?= x86_64-unknown-linux-gnu
# The release to attach binaries to. CI passes the tag via GITHUB_REF_NAME.
RELEASE_TAG ?= $(GITHUB_REF_NAME)

.PHONY: dist
dist: ## Build the release binary and stage it under dist/
	$(BASE_COMMAND) sh -c "rustup target add $(BINARY_TARGET) \
		&& cargo build --release --target $(BINARY_TARGET) \
		&& mkdir -p $(DIST_DIR) \
		&& cp target/$(BINARY_TARGET)/release/sbd $(DIST_DIR)/sbd-$(BINARY_TARGET)"

.PHONY: publish
publish: publish-binary publish-container ## Publish release artifacts (binary + container image)

.PHONY: publish-binary
publish-binary: dist ## Attach the release binary to the GitHub Release for RELEASE_TAG
	@test -n "$(RELEASE_TAG)" || { echo "RELEASE_TAG is empty (pass RELEASE_TAG=vX.Y.Z)"; exit 1; }
	gh release upload "$(RELEASE_TAG)" $(DIST_DIR)/sbd-$(BINARY_TARGET) --clobber

.PHONY: publish-crate
publish-crate: ## Publish to crates.io (opt-in; needs CARGO_REGISTRY_TOKEN)
	$(CONTAINER_ENGINE) run --rm \
		-v "$(CURDIR)":$(CONTAINER_WORK_DIR):Z -w $(CONTAINER_WORK_DIR) \
		-e CARGO_REGISTRY_TOKEN $(RUST_IMAGE) cargo publish

# --- Container image ---------------------------------------------------------
# Image tags: primary = <version>-<buildstamp>, plus alt tags
# <version> and <version>-<gitref>, never `latest`. The version is read from
# Cargo.toml (release-please's source of truth). Container tags drop any leading
# `v` (git tags are `vX.Y.Z`; image tags are `X.Y.Z`).
# CONTAINER_REGISTRY / CONTAINER_ORG / CONTAINER_REPO come from .config.include.mk.
CONTAINER_FILE ?= Containerfile
CONTAINER_URI ?= $(CONTAINER_REGISTRY)/$(CONTAINER_ORG)/$(CONTAINER_REPO)

BASE_VERSION := $(shell sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
GIT_REF := $(shell git rev-parse --short HEAD 2>/dev/null)
BUILDSTAMP := $(shell date -u +%Y%m%dT%H%MZ)
CONTAINER_VERSION := $(patsubst v%,%,$(BASE_VERSION))
# sbd is a CLI users pull directly, so rolling `1` (major) and `1.2`
# (major.minor) tags let them track patches/minors; the immutable `1.2.3`
# (+ buildstamp/ref) stays reproducible. `latest` is never published — see
# decisions/0002-container-image-tagging.md.
CONTAINER_MAJOR := $(word 1,$(subst ., ,$(CONTAINER_VERSION)))
CONTAINER_MAJOR_MINOR := $(CONTAINER_MAJOR).$(word 2,$(subst ., ,$(CONTAINER_VERSION)))
CONTAINER_BASE_TAG := $(CONTAINER_VERSION)-$(BUILDSTAMP)
CONTAINER_ALT_TAGS := $(CONTAINER_VERSION) $(CONTAINER_MAJOR_MINOR) $(CONTAINER_MAJOR) $(CONTAINER_VERSION)-$(GIT_REF)
CONTAINER_PRIMARY_TAG := $(CONTAINER_URI):$(CONTAINER_BASE_TAG)
# Architectures baked into the published image manifest.
CONTAINER_PLATFORMS ?= linux/amd64,linux/arm64

CONTAINER_LABELS := --label org.opencontainers.image.title="$(CONTAINER_REPO)"
CONTAINER_LABELS += --label org.opencontainers.image.version="$(CONTAINER_BASE_TAG)"
CONTAINER_LABELS += --label org.opencontainers.image.revision="$(GIT_REF)"
CONTAINER_LABELS += --label org.opencontainers.image.source="https://github.com/$(CONTAINER_ORG)/$(CONTAINER_REPO)"

.PHONY: build-container
build-container: ## Build the multi-arch image manifest (primary + alt tags, never `latest`)
	$(CONTAINER_ENGINE) build --platform $(CONTAINER_PLATFORMS) --manifest $(CONTAINER_PRIMARY_TAG) -f $(CONTAINER_FILE) $(CONTAINER_LABELS) .
	for TAG in $(CONTAINER_ALT_TAGS); do \
		$(CONTAINER_ENGINE) tag $(CONTAINER_PRIMARY_TAG) $(CONTAINER_URI):$${TAG}; \
	done

.PHONY: publish-container
publish-container: build-container ## Push the multi-arch manifest (all arches, primary + alt tags)
	$(CONTAINER_ENGINE) manifest push --all $(CONTAINER_PRIMARY_TAG) docker://$(CONTAINER_PRIMARY_TAG)
	for TAG in $(CONTAINER_ALT_TAGS); do \
		$(CONTAINER_ENGINE) manifest push --all $(CONTAINER_URI):$${TAG} docker://$(CONTAINER_URI):$${TAG}; \
	done

.PHONY: run
run: ## Run the CLI (make run ARGS="--help"); HUMAN USE — never in CI
	$(CARGO) run -- $(ARGS)

.PHONY: shell
shell: ## Open a shell in the build container
	$(BASE_COMMAND) sh

.PHONY: clean
clean: ## Remove build artifacts
	$(CARGO) clean
