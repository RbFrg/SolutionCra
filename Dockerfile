# syntax=docker/dockerfile:1

# 1. Front : build Vite -> /web/dist (version alignée sur web/.nvmrc)
FROM node:24-slim AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# 2. Back : dépendances en cache (cargo-chef), puis compilation
FROM lukemathwalker/cargo-chef:latest-rust-1.99.0 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
COPY --from=web /web/dist ./web/dist
RUN cargo build --release --bin cra-server

# 3. Exécution : distroless, utilisateur nonroot (uid 65532), aucun Node
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
ARG GIT_SHA=dev
ENV GIT_SHA=${GIT_SHA} CRA_BIND=0.0.0.0:8080
COPY --from=builder /app/target/release/cra-server /app/cra-server
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/app/cra-server"]
