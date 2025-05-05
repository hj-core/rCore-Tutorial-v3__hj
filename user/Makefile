# Variables__build
TARGET := riscv64gc-unknown-none-elf
MODE ?= release
ifeq ($(MODE), release)
	MODE_ARG = --release
else
	MODE_ARG = --debug
endif

APP_SRC_DIR := src/bin
APP_ELF_DIR := target/$(TARGET)/$(MODE)
APP_BIN_DIR := $(APP_ELF_DIR)
APPS := $(wildcard $(APP_SRC_DIR)/*.rs)
ELFS := $(patsubst $(APP_SRC_DIR)/%.rs, $(APP_ELF_DIR)/%, $(APPS))
BINS := $(patsubst $(APP_SRC_DIR)/%.rs, $(APP_BIN_DIR)/%.bin, $(APPS))

# Variables_binutils
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Variables_qemu
QEMU := qemu-riscv64

# Targets
build:
	@echo "> App List:"
	@$(foreach app, $(APPS), echo $(patsubst $(APP_SRC_DIR)/%, %, $(app));)
	@cargo build $(MODE_ARG)
	@$(foreach elf, $(ELFS), \
		$(OBJCOPY) $(elf) --strip-all -O binary $(patsubst $(APP_ELF_DIR)/%, $(APP_BIN_DIR)/%.bin, $(elf)); \
	)
.PHONY: build

clean:
	@cargo clean
.PHONY: clean

run: build
	@$(foreach elf, $(ELFS), \
		echo "> Running $(patsubst $(APP_ELF_DIR)/%, %, $(elf)):"; \
		$(QEMU) $(elf); \
	)
.PHONY: run