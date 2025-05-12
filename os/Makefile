# Variables__build
TARGET := riscv64gc-unknown-none-elf
MODE ?= release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := target/$(TARGET)/$(MODE)/os.bin
APPS_ASM := src/link_apps.S
USER_DIR := ../user

ifeq ($(MODE), release)
	MODE_ARG = --release
endif

# Variables__binutils
OBJCOPY := rust-objcopy --binary-architecture=riscv64
OBJDUMP := rust-objdump --arch-name=riscv64
OBJDUMP_ARGS ?= -h -d
OBJDUMP_TMP := target/$(TARGET)/$(MODE)/objdump.tmp

# Variables__qemu
BOOTLOADER := ../bootloader/rustsbi-qemu.bin
KERNEL_BASE := 0x80200000
QEMU := qemu-system-riscv64
QEMU_ARGS := -machine virt \
            -nographic \
			-bios $(BOOTLOADER) \
			-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_BASE)

# Variables_gdb
GDB := gdb-multiarch
GDB_ARGS := -ex 'file $(KERNEL_ELF)' \
            -ex 'set arch riscv:rv64' \
			-ex 'target remote localhost:1234'

# Targets
build: build_user
	@cargo build $(MODE_ARG)
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)
.PHONY: build

build_user:
	@$(MAKE) -C $(USER_DIR) MODE=$(MODE) build
.PHONY: build_user

clean: clean_user
	@cargo clean
	@rm -f $(KERNEL_BIN) $(OBJDUMP_TMP)
	@rm -f $(APPS_ASM)
.PHONY: clean

clean_user:
	@$(MAKE) -C $(USER_DIR) clean
.PHONY: clean_user

run: build
	@$(QEMU) $(QEMU_ARGS)
.PHONY: run

debug: build
	@tmux new-session -d \
	"$(QEMU) $(QEMU_ARGS) -s -S" && \
	tmux split-window -v "$(GDB) $(GDB_ARGS)" && \
	tmux -2 attach-session -d
.PHONY: debug

disasm: build
	@$(OBJDUMP) $(OBJDUMP_ARGS) $(KERNEL_ELF) > $(OBJDUMP_TMP)
	@vim $(OBJDUMP_TMP)
.PHONY: disasm