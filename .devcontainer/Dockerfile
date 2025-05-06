FROM ubuntu:24.04

# Install dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    pip \
    curl \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install Rust using rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.86.0 && \
    . $HOME/.cargo/env && \
    rustup toolchain install 1.75 1.79 && \
    cargo install cargo-certora-sbf && \
    cargo install just && \
    cargo certora-sbf --no-build

# Install Certora CLI
RUN pip install --break-system-packages certora-cli==7.28.0

ENTRYPOINT [ "/bin/bash" ]
