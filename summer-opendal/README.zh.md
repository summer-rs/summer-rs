
[![crates.io](https://img.shields.io/crates/v/spring-opendal.svg)](https://crates.io/crates/spring-opendal)
[![Documentation](https://docs.rs/spring-opendal/badge.svg)](https://docs.rs/spring-opendal)

## 依赖

```toml
spring-opendal = { version = "<version>" }
```

## 配置

```toml
[opendal]
scheme = "fs"                # OpenDAL支持的服务
options = { root = "/tmp" }  # 服务配置项，不同的scheme有不同的配置项
layers = []                  # Layer是拦截操作的机制
```

Layer的相关配置, 可参看[这个文档](https://docs.rs/opendal/latest/opendal/layers/index.html)

## Components

配置完以上配置项后，插件会自动注册一个 [`Op`](https://docs.rs/spring-opendal/latest/spring_opendal/type.Op.html) 客户端。该对象是 [`opendal::Operator`](https://docs.rs/opendal/latest/opendal/struct.Operator.html) 的别名。

```rust
pub type Op = Operator;
```

完整示例代码，参考 [`spring-opendal-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/spring-opendal-example)