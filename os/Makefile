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
			-device loader,file=$(KERNEL_ELF)

# Variables_gdb
GDB := gdb-multiarch
GDB_ARGS := -ex 'file $(KERNEL_ELF)' \
            -ex 'set arch riscv:rv64' \
			-ex 'target remote localhost:1234'

# Targets
.PHONY: build
build: build_user
	@cargo build $(MODE_ARG)
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

.PHONY: build_user
build_user:
	@$(MAKE) -C $(USER_DIR) MODE=$(MODE) build

.PHONY: clean
clean: clean_user
	@cargo clean
	@rm -f $(KERNEL_BIN) $(OBJDUMP_TMP)
	@rm -f $(APPS_ASM)

.PHONY: clean_user
clean_user:
	@$(MAKE) -C $(USER_DIR) clean

.PHONY: run
run: build
	@$(QEMU) $(QEMU_ARGS)

.PHONY: debug
debug: build
	@tmux new-session -d \
	"$(QEMU) $(QEMU_ARGS) -s -S" && \
	tmux split-window -h "$(GDB) $(GDB_ARGS)" && \
	tmux -2 attach-session -d

.PHONY: gdbc
gdbc:
	@$(GDB) $(GDB_ARGS)

.PHONY: gdbs
gdbs: build
	@$(QEMU) $(QEMU_ARGS) -s -S

.PHONY: disasm
disasm: build
	@$(OBJDUMP) $(OBJDUMP_ARGS) $(KERNEL_ELF) > $(OBJDUMP_TMP)
	@vim $(OBJDUMP_TMP)