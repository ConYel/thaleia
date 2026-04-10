# =============================================================================
# Thaleia Makefile - Rust Voice AI
# =============================================================================
#
# Architecture:
#   thaleia:builder   - Dev environment (Rust, OpenCode, all deps)
#   thaleia:prod      - Production runtime (binaries + minimal deps)
#
# Workflow:
#   1. make build-builder  - Build dev container (ONE TIME)
#   2. make build          - Build release binary (in container with cargo cache)
#   3. make test          - Run tests
#   4. make test-mcp     - Test MCP server
#   5. make shell         - Interactive shell
#
# Production:
#   make build-prod       - Build production image
#   make run-prod         - Run production container
#
# =============================================================================

# Configuration
CONTAINER := thaleia:builder
CONTAINER_PROD := thaleia:prod
CONTAINERFILE := Containerfile.dev
WORKSPACE := /home/devuser/workspace
PROJECT_DIR := $(CURDIR)
HOST_UID := $(shell id -u)
HOST_USER := $(shell whoami)
HOME_DIR := $(shell echo $$HOME)

# Default features for build
BUILD_FEATURES ?= kokoro,playback,rodio,sdl2-audio,whisper,mcp,vad

# Cache mounts for development (runs as devuser)
CACHE_MOUNTS := \
	-v $(HOME_DIR)/.cache/k:/home/devuser/.cache/k:Z \
	-v $(HOME_DIR)/.cache/ort.pyke.io:/home/devuser/.cache/ort.pyke.io:Z \
	-v $(HOME_DIR)/.cache/whisper:/home/devuser/.cache/whisper:Z \
	-v $(HOME_DIR)/.cargo:/home/devuser/.cargo:Z \
	-v $(HOME_DIR)/.config/pulse:/home/devuser/.config/pulse:Z \
	-v /run/user/$(HOST_UID)/pulse:/run/user/$(HOST_UID)/pulse:Z \
	-v $(PROJECT_DIR)/.config/opencode:/home/devuser/.config/opencode:Z

# Audio device mount (optional)
AUDIO_DEVICE := --device /dev/snd:/dev/snd:rw

.PHONY: help build-builder build build-mcp test shell test-mcp run build-prod run-prod clean fmt lint

# =============================================================================
# Help
# =============================================================================
help:
	@echo "Thaleia - Rust Voice AI"
	@echo ""
	@echo "=== Setup (one time) ==="
	@echo "  make build-builder   Build dev container with all deps"
	@echo ""
	@echo "=== Development ==="
	@echo "  make build            Build release binary (FEATURES=$(BUILD_FEATURES))"
	@echo "  make test             Run cargo tests"
	@echo "  make shell            Interactive shell in container"
	@echo "  make test-mcp         Test MCP server"
	@echo "  make run              Run arbitrary command"
	@echo ""
	@echo "=== Code Quality ==="
	@echo "  make fmt              Format code with cargo fmt"
	@echo "  make lint             Run clippy linter"
	@echo ""
	@echo "=== Production ==="
	@echo "  make build-prod       Build production image"
	@echo "  make run-prod         Run production container"
	@echo ""
	@echo "=== Utils ==="
	@echo "  make clean            Clean build artifacts"
	@echo ""
	@echo "=== Variables ==="
	@echo "  FEATURES=$(BUILD_FEATURES)"

# =============================================================================
# Build dev container (one time)
# =============================================================================
build-builder:
	@echo "Building thaleia:builder..."
	podman build --target builder -t $(CONTAINER) -f $(CONTAINERFILE) $(PROJECT_DIR)
	@echo "Done! Run 'make build' to build the binary."

# Check if container exists, build if not
_check:
	@if ! podman image exists $(CONTAINER) 2>/dev/null; then \
		echo "Container $(CONTAINER) not found. Running 'make build-builder'..."; \
		$(MAKE) build-builder; \
	fi

# =============================================================================
# Helper: Run command in container (as devuser)
# =============================================================================
define run-in-container
podman run --rm \
	--userns=keep-id \
	--security-opt label=disable \
	--group-add keep-groups \
	-v $(PROJECT_DIR):$(WORKSPACE):Z \
	$(CACHE_MOUNTS) \
	-e XDG_RUNTIME_DIR=/run/user/$(HOST_UID) \
	-e PULSE_SERVER=unix:/run/user/$(HOST_UID)/pulse/native \
	-e SDL_AUDIODRIVER=pulse \
	-e ESPEEAKNG_DATA_DIR=/usr/lib/x86_64-linux-gnu/espeak-ng-data \
	-e PIPER_ESPEAKNG_DATA_DIRECTORY=/usr/lib/x86_64-linux-gnu/espeak-ng-data \
	$(CONTAINER) \
	sh -c "cd $(WORKSPACE) && $(1)"
endef

# =============================================================================
# Build
# =============================================================================
build: _check
	@echo "Building with features: $(BUILD_FEATURES)"
	$(call run-in-container,cargo build --release --features=$(BUILD_FEATURES))
	@echo "Binary: $(PROJECT_DIR)/target/release/thaleia"

# Build MCP only (faster, fewer features)
build-mcp: _check
	$(call run-in-container,cargo build --release -p thaleia-mcp)

# =============================================================================
# Test
# =============================================================================
test: _check
	$(call run-in-container,cargo test)

# =============================================================================
# Code Quality
# =============================================================================
fmt: _check
	@echo "Formatting code..."
	$(call run-in-container,cargo fmt --all)

lint: _check
	@echo "Running clippy linter..."
	$(call run-in-container,cargo clippy --all-targets --all-features -- -D warnings)

# =============================================================================
# Shell
# =============================================================================
shell: _check
	@echo "Starting shell in $(CONTAINER)..."
	@echo "Mounts: project, cargo cache, model caches"
	podman run --rm -it \
		--userns=keep-id \
		--security-opt label=disable \
		--group-add keep-groups \
		-v $(PROJECT_DIR):$(WORKSPACE):Z \
		$(CACHE_MOUNTS) \
		$(AUDIO_DEVICE) \
		-e SDL_AUDIODRIVER=pulse \
		-e ESPEEAKNG_DATA_DIR=/usr/lib/x86_64-linux-gnu/espeak-ng-data \
		-e PIPER_ESPEAKNG_DATA_DIRECTORY=/usr/lib/x86_64-linux-gnu/espeak-ng-data \
		-e XDG_RUNTIME_DIR=/run/user/$(HOST_UID) \
		-e PULSE_SERVER=unix:/run/user/$(HOST_UID)/pulse/native \
		$(CONTAINER) \
		sh -c "cd $(WORKSPACE) && bash"

# =============================================================================
# Test MCP Server
# =============================================================================
test-mcp: _check
	@echo "Testing MCP server..."
	@{ \
		echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'; \
		sleep 1; \
		echo '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'; \
	} | timeout 30 podman run --rm -i \
		--userns=keep-id \
		--security-opt label=disable \
		-v $(PROJECT_DIR):$(WORKSPACE):Z \
		$(CACHE_MOUNTS) \
		-e XDG_RUNTIME_DIR=/run/user/$(HOST_UID) \
		-e PULSE_SERVER=unix:/run/user/$(HOST_UID)/pulse/native \
		-e SDL_AUDIODRIVER=pulse \
		-e ESPEEAKNG_DATA_DIR=/usr/lib/x86_64-linux-gnu/espeak-ng-data \
		-e PIPER_ESPEAKNG_DATA_DIRECTORY=/usr/lib/x86_64-linux-gnu/espeak-ng-data \
		$(CONTAINER) \
		sh -c "cd $(WORKSPACE) && ./target/release/thaleia-mcp --mode standard" \
		2>/dev/null | grep -q "protocolVersion" \
		&& echo "✅ MCP server test passed" \
		|| echo "❌ MCP server test failed"

# =============================================================================
# Run arbitrary command
# =============================================================================
run: _check
	$(call run-in-container,$(CMD))

# =============================================================================
# Production
# =============================================================================
build-prod:
	@echo "Building production image..."
	podman build \
		--target production \
		-t $(CONTAINER_PROD) \
		-f $(CONTAINERFILE) \
		-v $(HOME_DIR)/.cargo:/root/.cargo:Z \
		$(PROJECT_DIR)
	@echo "Production image ready: $(CONTAINER_PROD)"

run-prod:
	@echo "Running production container..."
	podman run --rm -it \
		--userns=keep-id \
		--security-opt label=disable \
		--group-add keep-groups \
		$(CACHE_MOUNTS) \
		$(AUDIO_DEVICE) \
		-e XDG_RUNTIME_DIR=/run/user/$(HOST_UID) \
		-e PULSE_SERVER=unix:/run/user/$(HOST_UID)/pulse/native \
		-e SDL_AUDIODRIVER=pulse \
		$(CONTAINER_PROD)

# =============================================================================
# Clean
# =============================================================================
clean:
	@echo "Cleaning build artifacts..."
	rm -rf $(PROJECT_DIR)/target
	@echo "Done!"
