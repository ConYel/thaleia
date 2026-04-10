# Thaleia Test Scripts

These scripts are for testing Thaleia in the container. Copy to `/usr/local/bin/` in the container or mount them at runtime.

## Installation

### Option 1: Copy during build (in Containerfile)
```dockerfile
COPY scripts/container-scripts/ /usr/local/bin/
```

### Option 2: Mount at runtime (in Makefile)
```makefile
-v $(PROJECT_DIR)/scripts/container-scripts:/usr/local/bin:Z
```

---

## Scripts

### thaleia-test
Main test suite - shows available test commands and audio devices.

```bash
thaleia-test
```

### thaleia-tts-test [TEXT]
Quick TTS test - speaks the given text (default: "Hello")

```bash
thaleia-tts-test "Hello world"
thaleia-tts-test  # uses default "Hello"
```

### thaleia-stt-test [TIMEOUT]
Quick STT test - listens for TIMEOUT seconds (default: 5)

```bash
thaleia-stt-test 10
thaleia-stt-test  # uses default 5 seconds
```

---

## Usage in Container

```bash
# Test TTS
thaleia-tts-test "Hello from Thaleia!"

# Test STT (speak now!)
thaleia-stt-test 5

# Show test menu
thaleia-test
```
