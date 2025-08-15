# syntax=docker/dockerfile:1

# EXAMPLE:
# docker build -t imageName:version .
# docker run -it --rm -v path/to/local/repo:/rcore-repo imageName

######## STAGE 1: Build QEMU
FROM ubuntu:25.04 AS build_qemu
# Download QEMU
ARG QEMU_VERSION=9.2.4
ADD https://download.qemu.org/qemu-${QEMU_VERSION}.tar.xz .
# Install QEMU build dependencies
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    build-essential \
    libglib2.0-dev \
    libpixman-1-dev \
    git \
    ninja-build \
    python3-venv \
    zlib1g-dev
# Build QEMU
RUN tar xf qemu-${QEMU_VERSION}.tar.xz \
    && cd qemu-${QEMU_VERSION} \
    && ./configure --target-list=riscv64-softmmu,riscv64-linux-user \
    && make -j$(nproc)

######## STAGE FINAL:
FROM ubuntu:25.04 AS rcore
# Copy QEMU and configure environmental variables
ARG QEMU_VERSION=9.2.4
COPY --from=build_qemu qemu-${QEMU_VERSION}/build qemu-${QEMU_VERSION}/build
ENV PATH=$PATH:/qemu-${QEMU_VERSION}/build
# Install dependencies and tools
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    build-essential \
    curl \
    gdb-multiarch \
    git \
    libglib2.0-dev \
    libpixman-1-dev \
    tmux \
    vim
# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal \
    && . "$HOME/.cargo/env" \
    && rustup target add riscv64gc-unknown-none-elf \
    && rustup component add llvm-tools-preview \
    && cargo install cargo-binutils
# Add a volume for connecting the local repository in host
VOLUME /rcore-repo