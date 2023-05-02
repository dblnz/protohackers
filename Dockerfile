FROM rust:1.68.0-slim-bullseye AS builder

ARG SOL
ARG APP_NAME=protohackers

RUN echo "Preparing app ${APP_NAME}"

WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/app/target \
		--mount=type=cache,target=/usr/local/cargo/registry \
		--mount=type=cache,target=/usr/local/cargo/git \
		--mount=type=cache,target=/usr/local/rustup \
		set -eux; \
		rustup install stable; \
	 	cargo build --release --features $SOL; \
		objcopy --compress-debug-sections target/release/$APP_NAME ./$APP_NAME

################################################################################
FROM debian:11.3-slim
ARG APP_NAME=protohackers
RUN echo "Preparing app ${APP_NAME}"

RUN set -eux; \
		export DEBIAN_FRONTEND=noninteractive; \
	  	apt update; \
		apt install --yes --no-install-recommends bind9-dnsutils iputils-ping iproute2 curl ca-certificates htop netcat; \
		apt clean autoclean; \
		apt autoremove --yes; \
		rm -rf /var/lib/{apt,dpkg,cache,log}/; \
		echo "Installed base utils!"

WORKDIR app

EXPOSE 8080

COPY --from=builder /app/$APP_NAME ./$APP_NAME
ENV envValue=${APP_NAME}

CMD ./${envValue}