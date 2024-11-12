# The Rust kernel module name
MODULE_NAME := ttyprintk_driver

# Path to the Linux kernel source (set this to the location of your kernel source)
KERNEL_SRC := /path/to/linux-kernel-source

# Path to your kernel module source files
RUST_SRC := src

# Path to the target output directory (can be 'target' by default)
OUT_DIR := target

# Set the architecture to your target architecture (x86_64 as an example)
ARCH := x86_64

# Kernel build directory
KERNEL_BUILD_DIR := $(KERNEL_SRC)/build

# Set the toolchain for cross-compilation if necessary
CARGO_TARGET := x86_64-unknown-linux-gnu

# Default target to build the kernel module
all: $(MODULE_NAME).ko

# Rule to build the kernel module
$(MODULE_NAME).ko: $(RUST_SRC)/*
	# Build the Rust kernel module using cargo
	cargo build --release --target=$(CARGO_TARGET)

# Make sure to include kernel build infrastructure for module loading
install: $(MODULE_NAME).ko
	sudo insmod $(OUT_DIR)/$(MODULE_NAME).ko

# Rule to clean the build artifacts
clean:
	cargo clean

# Uninstall the kernel module
uninstall:
	sudo rmmod $(MODULE_NAME)

# Load the kernel module
load: $(MODULE_NAME).ko
	sudo insmod $(OUT_DIR)/$(MODULE_NAME).ko

# Display the status of the kernel module
status:
	lsmod | grep $(MODULE_NAME)

.PHONY: all clean install uninstall load status
