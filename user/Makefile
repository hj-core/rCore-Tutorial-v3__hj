# Variables__build
TARGET := riscv64gc-unknown-none-elf
MODE ?= release
ifeq ($(MODE), release)
	MODE_ARG = --release
endif

APP_SRC_DIR := src/bin
APP_ELF_DIR := target/$(TARGET)/$(MODE)
APP_BIN_DIR := $(APP_ELF_DIR)

TEST ?= 0

APPS := $(wildcard $(APP_SRC_DIR)/*.rs)
ifeq ($(TEST), 0)
	APPS := $(filter-out $(wildcard $(APP_SRC_DIR)/test_*.rs), $(APPS))
endif

ELFS := $(patsubst $(APP_SRC_DIR)/%.rs, $(APP_ELF_DIR)/%, $(APPS))
BINS := $(patsubst $(APP_SRC_DIR)/%.rs, $(APP_BIN_DIR)/%.bin, $(APPS))

# Variables_binutils
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Variables_qemu
QEMU := qemu-riscv64

# Targets
.PHONY: build
build:
	@python3 build.py $(MODE_ARG) --bin

.PHONY: clean
clean:
	@cargo clean
	@rm src/linker.ld

.PHONY: run
run: build
	@$(foreach elf, $(ELFS), \
		echo "> Running $(patsubst $(APP_ELF_DIR)/%, %, $(elf)):"; \
		$(QEMU) $(elf); \
	)