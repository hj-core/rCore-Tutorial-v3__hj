# Variables__build
TARGET := riscv64gc-unknown-none-elf
MODE ?= release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := target/$(TARGET)/$(MODE)/os.bin

ifeq ($(MODE), release)
	MODE_ARG = --release
else
	MODE_ARG = --debug
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
build:
	@cargo build $(MODE_ARG)
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)
.PHONY: build

clean:
	@cargo clean
	@rm -f $(KERNEL_BIN) $(OBJDUMP_TMP)
.PHONY: clean

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