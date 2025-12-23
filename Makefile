# Makefile for cross-compiling to Raspberry Pi

# Variables
TARGET_ARM64 = aarch64-unknown-linux-gnu
TARGET_ARM32 = armv7-unknown-linux-gnueabihf
BINARY_NAME = weight-sensor-pump-controller
TARGET_WEIGHT ?= 50.0
BUILD_MODE ?= release

# Default target
.PHONY: all
all: build-arm64

# Install cross-compilation toolchain
.PHONY: setup
setup:
	rustup target add $(TARGET_ARM64)
	rustup target add $(TARGET_ARM32)
	sudo apt-get update
	sudo apt-get install -y gcc-aarch64-linux-gnu gcc-arm-linux-gnueabihf

# Build for Raspberry Pi 4 (ARM64)
.PHONY: build-arm64
build-arm64:
	TARGET_WEIGHT=$(TARGET_WEIGHT) cargo build --target $(TARGET_ARM64) --$(BUILD_MODE)

# Build for Raspberry Pi 3 and older (ARM32)
.PHONY: build-arm32
build-arm32:
	TARGET_WEIGHT=$(TARGET_WEIGHT) cargo build --target $(TARGET_ARM32) --$(BUILD_MODE)

# Build both ARM targets
.PHONY: build-all
build-all: build-arm64 build-arm32

# Build for development (local)
.PHONY: build-dev
build-dev:
	TARGET_WEIGHT=$(TARGET_WEIGHT) cargo build

# Build with Docker
.PHONY: build-docker
build-docker:
	docker build -f Dockerfile.cross -t weight-sensor-cross .
	docker run --rm -v $(PWD)/target:/workspace/target weight-sensor-cross

# Run tests
.PHONY: test
test:
	cargo test

# Run property-based tests
.PHONY: test-props
test-props:
	cargo test --features quickcheck

# Clean build artifacts
.PHONY: clean
clean:
	cargo clean

# Copy binary to Raspberry Pi (requires SSH setup)
.PHONY: deploy-arm64
deploy-arm64: build-arm64
	scp target/$(TARGET_ARM64)/$(BUILD_MODE)/$(BINARY_NAME) pi@raspberrypi.local:~/

.PHONY: deploy-arm32
deploy-arm32: build-arm32
	scp target/$(TARGET_ARM32)/$(BUILD_MODE)/$(BINARY_NAME) pi@raspberrypi.local:~/

# Check binary architecture
.PHONY: check-arch-arm64
check-arch-arm64: build-arm64
	file target/$(TARGET_ARM64)/$(BUILD_MODE)/$(BINARY_NAME)

.PHONY: check-arch-arm32
check-arch-arm32: build-arm32
	file target/$(TARGET_ARM32)/$(BUILD_MODE)/$(BINARY_NAME)

# Create release package
.PHONY: package
package: build-all
	mkdir -p dist
	cp target/$(TARGET_ARM64)/$(BUILD_MODE)/$(BINARY_NAME) dist/$(BINARY_NAME)-arm64
	cp target/$(TARGET_ARM32)/$(BUILD_MODE)/$(BINARY_NAME) dist/$(BINARY_NAME)-arm32
	cp README.md dist/
	cp config.toml.example dist/
	tar -czf dist/$(BINARY_NAME)-$(shell date +%Y%m%d).tar.gz -C dist .

# Generate default config file
.PHONY: config
config:
	echo 'target_weight = 50.0' > config.toml.example
	echo '' >> config.toml.example
	echo '[gpio]' >> config.toml.example
	echo 'dt_pin = 5' >> config.toml.example
	echo 'sck_pin = 6' >> config.toml.example
	echo 'pump_pin = 18' >> config.toml.example
	echo 'button_pin = 2' >> config.toml.example
	echo '' >> config.toml.example
	echo '[sensor]' >> config.toml.example
	echo 'calibration_factor = 1.0' >> config.toml.example
	echo 'moving_average_window = 5' >> config.toml.example
	echo 'max_weight = 100.0' >> config.toml.example

# Help
.PHONY: help
help:
	@echo "Available targets:"
	@echo "  setup        - Install cross-compilation toolchain"
	@echo "  build-arm64  - Build for Raspberry Pi 4+ (ARM64)"
	@echo "  build-arm32  - Build for Raspberry Pi 3 and older (ARM32)"
	@echo "  build-all    - Build for both ARM targets"
	@echo "  build-dev    - Build for development (local)"
	@echo "  build-docker - Build using Docker"
	@echo "  test         - Run unit tests"
	@echo "  test-props   - Run property-based tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  deploy-arm64 - Deploy ARM64 binary to Raspberry Pi via SSH"
	@echo "  deploy-arm32 - Deploy ARM32 binary to Raspberry Pi via SSH"
	@echo "  check-arch-* - Check binary architecture"
	@echo "  package      - Create release package"
	@echo "  config       - Generate example config file"
	@echo "  help         - Show this help"
	@echo ""
	@echo "Environment variables:"
	@echo "  TARGET_WEIGHT - Target weight in grams (default: 50.0)"
	@echo "  BUILD_MODE    - Build mode: debug or release (default: release)"