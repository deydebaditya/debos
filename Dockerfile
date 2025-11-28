# DebOS Build Environment
# Supports both x86_64 and AArch64 targets

FROM rust:latest

# Install build dependencies
RUN apt-get update && apt-get install -y \
    # QEMU for both architectures
    qemu-system-x86 \
    qemu-system-arm \
    # Build tools
    nasm \
    mtools \
    xorriso \
    clang \
    lld \
    llvm-dev \
    libclang-dev \
    # Filesystem tools
    e2fsprogs \
    dosfstools \
    # Utilities
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install Rust nightly with required components
RUN rustup toolchain install nightly \
    && rustup default nightly \
    && rustup component add rust-src llvm-tools-preview clippy rustfmt \
    && rustup target add x86_64-unknown-none aarch64-unknown-none

# Set working directory
WORKDIR /debos

# Copy project files
COPY . .

# Default command: show available targets
CMD ["make", "help"]
