# syntax=docker/dockerfile:1
# 多阶段构建：node 构建前端 → rust(alpine/musl) 静态编译后端 → scratch 最小运行镜像。
# 最终镜像仅含：静态链接的 server 二进制 + CA 证书 ≈ 20MB（压缩后 ~7MB）。
#
# 缓存：BuildKit cache mount 持久化 cargo registry（依赖下载）与 target（编译产物），
# 源码改动只重编译变更的 crate（约 1 分钟）；依赖未变时层缓存直接命中。

# ---------- 阶段 1：前端静态资源 ----------
FROM node:22-alpine AS frontend
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm \
    npm ci
COPY frontend/ ./
RUN npm run build

# ---------- 阶段 2：Rust 后端编译 ----------
# alpine 版工具链默认 musl target → 产物静态链接，scratch 可直接运行。
# 锁定 1.96 与本地 stable 一致（1.97 存在类型推断行为差异）。
FROM rust:1.96-alpine AS builder
# gcc/musl-dev：编译 sqlx 内置的 bundled SQLite C 源码；ca-certificates 供最终阶段复制
RUN apk add --no-cache gcc musl-dev ca-certificates
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY --from=frontend /app/dist ./frontend/dist
# target 挂 cache mount：产物跨构建持久化，增量编译生效
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release && \
    cp /app/target/release/server /usr/local/bin/server

# 数据目录归属运行时 uid 1000（named volume 首次挂载会继承所有权）
RUN mkdir -p /app/data && chown -R 1000:1000 /app/data

# ---------- 阶段 3：scratch 运行镜像 ----------
FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /usr/local/bin/server /app/server
COPY --from=builder --chown=1000:1000 /app/data /app/data
WORKDIR /app
USER 1000:1000
EXPOSE 3000
ENTRYPOINT ["/app/server"]
