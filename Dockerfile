FROM ubuntu:22.04 AS builder

# Install required dependencies
RUN set -eux; \
		apt update; \
		apt install -y --no-install-recommends \
			curl ca-certificates gcc clang \
			;

# Install rustup
RUN set -eux; \
		curl --location --fail \
			"https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init" \
			--output rustup-init; \
		chmod +x rustup-init; \
		./rustup-init -y --no-modify-path --default-toolchain stable; \
		rm rustup-init;

# Add rustup to path, check that it works
ENV PATH=${PATH}:/root/.cargo/bin
RUN set -eux; \
	rustup --version;

# Copy sources and build them
WORKDIR /app
COPY src src
COPY Cargo.toml Cargo.lock ./
#   👇 and here's some caches!
RUN --mount=type=cache,target=/root/.rustup \
    --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
	--mount=type=cache,target=/app/target \
	set -eux; \
	rustup default stable; \
	cargo build --release; \
	cp target/release/fresh-rust /app/fresh-rust


FROM ubuntu:22.04

RUN set -eux; \
    apt update; \
    apt install -y --no-install-recommends \
    curl unzip ca-certificates bash

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN set -eux; \
    curl -fsSL https://deno.land/x/install/install.sh | bash

ENV PATH=${PATH}:/root/.deno/bin

RUN set -eux; \
    deno upgrade --version 1.32.4

COPY --from=builder /app/fresh-rust /app/fresh-rust

CMD ["/app/fresh-rust"]