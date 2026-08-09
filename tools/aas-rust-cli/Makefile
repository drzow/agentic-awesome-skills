.PHONY: all build test clean install help

BIN_DIR := $(HOME)/.local/bin
BINARY  := target/release/aas
LINK    := $(BIN_DIR)/aas

all: build

build:
	@echo "==> Building aas..."
	cargo build --release

test:
	@echo "==> Running tests..."
	cargo test

clean:
	@echo "==> Cleaning..."
	cargo clean
	rm -f $(LINK) 2>/dev/null || true

install: build
	@echo "==> Installing to $(BIN_DIR)..."
	mkdir -p $(BIN_DIR)
	cp $(BINARY) $(LINK)
	chmod +x $(LINK)
	@echo "Installed to $(LINK)"
	@if ! echo $$PATH | grep -q "$(BIN_DIR)"; then \
		echo ""; \
		echo "Warning: $(BIN_DIR) is not in PATH."; \
		echo "Add this to your shell config (~/.bashrc, ~/.zshrc):"; \
		echo '  export PATH="$$HOME/.local/bin:$$PATH"'; \
	fi

help:
	@echo "Targets:"
	@echo "  build    Compile release binary (cargo build --release)"
	@echo "  test     Run cargo test"
	@echo "  clean    Remove build artifacts and installed binary"
	@echo "  install  Build and copy to $(BIN_DIR)/aas"
