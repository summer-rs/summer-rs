# Agents 指南

本文档面向在本仓库中协助开发或审查代码的自动化代理（AI Agent）与人类贡献者，说明项目结构、常用命令与约定。

## 项目是什么

**summer-rs** 是受 Java Spring Boot 启发的 Rust 应用框架：约定优于配置、可插拔插件、TOML 配置。核心 crate 为 `summer`；生态插件以 `summer-*` 命名（如 `summer-web`、`summer-sqlx`）。

- 官方文档：<https://summer-rs.github.io/>
- 仓库：<https://github.com/summer-rs/summer-rs>

## 工作区布局

| 路径 | 说明 |
|------|------|
| `summer/` | 核心：配置、插件、组件与 `App` |
| `summer-macros/` | 过程宏（如 `#[auto_config]`） |
| `summer-web/`、`summer-grpc/`、`summer-job/` 等 | 官方插件 |
| `contrib-plugins/`（独立仓库 [contrib-plugins](https://github.com/summer-rs/contrib-plugins)） | 社区维护的 summer 插件；对 `summer` 等依赖使用 **crates.io 版本号**，**不参与** `summer-rs` 的 Cargo workspace；其中 **`summer-ext-macros`** 为 Sa-Token / Pub/Sub 等扩展提供过程宏（由 **`summer-sa-token`**、**`summer-pubsub`** 重导出，应用侧一般无需直接依赖） |
| `summer-rs/contrib-plugins` | **不在本仓库中提交**：本地开发可在 `summer-rs` 根目录建指向 `../contrib-plugins` 的符号链接（已被 `.gitignore` 忽略），以便 `examples/integrations/plugin-example` 等 `path = "../../../contrib-plugins/..."` 能解析；CI 通过单独 checkout 该仓库到 `contrib-plugins/` |
| `examples/<主题>/*` | 示例应用（按子目录分组；工作区成员；`default-members` 不含示例） |
| `docs/` | 站点与文档源码（Zola 等） |

插件需实现 `Plugin`；配置需实现 `Configurable`；可作为组件注入的类型需 `Clone`。详见 `summer/Plugin.md` 与 `summer/DI.md`。

## 构建与检查

在仓库根目录执行：

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all -- --check
```

修改某 crate 时，可只针对该路径缩短迭代，例如：

```bash
cargo test -p summer
cargo test -p summer-web
```

## 运行示例（Examples）

默认从**进程当前工作目录**读取 `./config/app.toml`（见 `summer` 中 `App` 的配置路径约定）。因此运行某个示例时，应**先 `cd` 到该 example 的包根目录**（即含有 `config/` 与 `Cargo.toml` 的目录），再执行 `cargo run`，避免在仓库根目录用 `cargo run -p …` 导致配置缺失或字段反序列化失败。

许多示例带有 **`compose.yaml`**，用于在本机拉起该示例所需的外部依赖（数据库、Redis、Kafka、Jaeger 等）。典型流程：

```bash
cd examples/<分组>/<示例目录>
docker compose up -d    # 若存在 compose.yaml，按需启动依赖
cargo run               # 多二进制示例需加 --bin，例如 grpc-example
# …
docker compose down     # 用完后可关闭
```

**Stream 示例**：`examples/stream/` 下的 `stream-file-example`、`stream-kafka-example`、`stream-redis-example` 依赖文件流、Kafka（常见为 compose 里的 Redpanda）或 Redis，镜像与就绪等待较重。**全量冒烟、批量跑 examples 或 Agent 自检时默认跳过**；仅在任务明确要求或需要验证 `summer-stream` 时再进入对应目录，按该目录 `README.md` 与 `compose.yaml` 单独运行（多二进制需 `cargo run --bin producer` / `--bin consumer`）。

## 修改时的约定

- **范围**：优先只改与任务直接相关的文件与 crate，避免无关格式化或大范围重排。
- **风格**：与相邻代码保持一致（命名、错误处理、`async`/trait 用法）；新代码在提交前应能通过 `cargo fmt` 与 `clippy`。
- **依赖**：工作区统一版本在根目录 `Cargo.toml` 的 `[workspace.dependencies]`；子 crate 用 `workspace = true` 引用。
- **文档**：除非任务要求，不要随意新增或大面积改写用户文档；核心行为变更时考虑同步 `CHANGELOG` 或对应 README。

## 快速定位

- 应用入口与插件装配：`summer` 中的 `App`、`AppBuilder`。
- Web 路由与提取器：`summer-web`。
- 数据库：`summer-sqlx`、`summer-sea-orm`、`summer-postgres` 等按场景选用。

若不确定行为，优先阅读对应 crate 下的 `README.md` 及 `summer/` 内专题 Markdown（如 `Config.md`、`COMPONENT_MACRO.md`）。
