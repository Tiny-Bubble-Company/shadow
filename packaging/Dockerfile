# Shadow as a sidecar container.
#   docker build -f packaging/Dockerfile -t shadow .
#   docker run -d -p 127.0.0.1:8877:8877 -v shadow-data:/data shadow
# Point your app at http://shadow:8877/gw/<provider>/... (gateway mode, no CA needed), or
# use HTTPS_PROXY=http://shadow:8877 and trust http://shadow:8877/shadow/ca.pem.
FROM rust:1.88-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p shadow-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --home /data shadow && mkdir -p /data && chown shadow /data
COPY --from=build /src/target/release/shadow /usr/local/bin/shadow
USER shadow
ENV SHADOW_HOME=/data RUST_LOG=info
VOLUME ["/data"]
EXPOSE 8877
HEALTHCHECK --interval=30s --timeout=3s CMD ["shadow", "status", "--quiet"]
ENTRYPOINT ["shadow"]
CMD ["run", "--listen", "0.0.0.0:8877"]
