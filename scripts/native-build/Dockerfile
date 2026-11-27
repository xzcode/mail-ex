FROM node:24-bookworm-slim
RUN apt-get -o Acquire::Retries=1 -o Acquire::http::Timeout=20 update && DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::Retries=1 -o Acquire::http::Timeout=25 install -y --no-install-recommends build-essential pkg-config ca-certificates curl file libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev xvfb xauth dbus-x11 fonts-dejavu-core && rm -rf /var/lib/apt/lists/*
ENV PATH=/rust/bin:$PATH
ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/target
ENV CARGO_PROFILE_DEV_DEBUG=0
ENV CARGO_PROFILE_DEV_INCREMENTAL=false
ENV CARGO_BUILD_JOBS=1
WORKDIR /work
