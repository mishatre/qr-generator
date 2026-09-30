FROM local/vrunner:8.3.24.1667

# The base image is built by /Users/mt/dev/1C/docker (src/build-vrunner.sh) and
# carries 1C:Enterprise 8.3.24.1667 (client, server and ibcmd), OneScript 2.1.0,
# opm 1.4.1 and vanessa-runner 3.0.0. It has no Rust, no Python and no C
# toolchain, so nothing in this project can be built until this file has run.
# `local/onescript-dev-base:2.1.0` is referenced by the sibling Dockerfiles as a
# comment only; it is not produced by the image repository.

USER root

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y \
        bash \
        ca-certificates \
        curl \
        git \
        openssh-client \
        procps \
        inetutils-tools \
        iproute2 \
        net-tools \
        dmidecode \
        build-essential \
        pkg-config \
        zip \
        unzip \
        python3 \
        gcc-mingw-w64-x86-64 \
    && rm -rf /var/lib/apt/lists/*

# inetutils-tools guarantees `hostname`, iproute2/net-tools provide `ip` and
# `ifconfig`, dmidecode answers the hardware queries. The 1C licence check shells
# out to all of them to assemble its system report; when they are missing the
# platform rejects a valid developer licence with "Использование лицензии для
# разработчиков запрещено" while the launcher logs "/sbin/ip: not found".
#
# gcc-mingw-w64-x86-64 is the linker for the Windows cross-target
# (x86_64-pc-windows-gnu). A free toolchain is the reason this component is Rust
# rather than C++: the C++ route needs an MSVC licence to build the Windows DLL.

# Rust is installed system-wide rather than under /root so that every build step
# and any non-root container user sees the same toolchain. The pinned version and
# the target list live in /workspace/rust-toolchain.toml; this install only
# provides the bootstrap, and the rustup targets here are the ones the toolchain
# file declares, so the two cannot drift apart silently. `--no-modify-path` keeps
# rustup from editing shell profiles; PATH is set explicitly below.
#
# clippy and rustfmt are the workspace's lints and formatter. rust-analyzer is not
# a build tool at all: it is here so that the VS Code extension has a server built
# from the same commit as the pinned rustc. The extension's own bundled server
# refuses a toolchain it considers too old, and without the component the image
# would make every developer watch that warning and then download the component by
# hand. The toolchain file declares the same list, so `cargo` would fetch it anyway;
# installing it here just keeps the download inside the image build.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain 1.92.0 --no-modify-path \
    && rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-gnu \
    && rustup component add clippy rustfmt rust-analyzer \
    && cargo --version \
    && rustc --version

# 1cv8 and 1cv8c link against WebKitGTK 4.0, libsoup 2.4 and javascriptcoregtk
# 4.0, which Ubuntu 24.04 no longer ships. The script installs them from Ubuntu
# 22.04 and points the platform's bundled libstdc++ at the system one, so the
# client binaries at least load. A 1C licence is still required to run them.
COPY tools/1c-platform/install-client-runtime.sh /usr/local/bin/install-1c-client-runtime
RUN chmod +x /usr/local/bin/install-1c-client-runtime \
    && /usr/local/bin/install-1c-client-runtime

WORKDIR /workspace

ENTRYPOINT []
CMD ["sleep", "infinity"]
