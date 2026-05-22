# summer-rs：Rust 工作区 + code-server（CNB 云原生开发）
# 参考：https://docs.cnb.cool/zh/workspaces/custom-dev-env.html
FROM rust:bookworm

# 原生依赖：OpenSSL、gRPC 示例的 protoc、SSH/常用工具
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    wget \
    unzip \
    openssh-server \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# 默认工具链组件（与 AGENTS.md 中的 fmt/clippy 一致）
RUN rustup component add rustfmt clippy

# code-server 与常用扩展（在 CNB 文档示例基础上改为 Rust 向）
RUN curl -fsSL https://code-server.dev/install.sh | sh \
    && code-server --install-extension rust-lang.rust-analyzer \
    && code-server --install-extension vadimcn.vscode-lldb \
    && code-server --install-extension tamasfe.even-better-toml \
    && code-server --install-extension redhat.vscode-yaml \
    && code-server --install-extension waderyan.gitblame \
    && code-server --install-extension mhutchie.git-graph \
    && code-server --install-extension donjayamanne.githistory \
    && echo done

ENV LANG=C.UTF-8
ENV LANGUAGE=C.UTF-8
