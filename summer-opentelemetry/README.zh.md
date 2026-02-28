
[![crates.io](https://img.shields.io/crates/v/spring-opentelemetry.svg)](https://crates.io/crates/spring-opentelemetry)
[![Documentation](https://docs.rs/spring-opentelemetry/badge.svg)](https://docs.rs/spring-opentelemetry)

## 依赖

```toml
spring-opentelemetry = "<version>"
```

OTEL默认使用[W3C格式](https://github.com/w3c/trace-context)传递链路追踪的上下文信息。

可选的features: 
* `jaeger`: 使用[jaeger格式](https://www.jaegertracing.io/docs/1.18/client-libraries/#propagation-format)透传上下文
* `zipkin`: 使用[zipkin格式](https://github.com/openzipkin/b3-propagation)透传上下文
* `more-resource`: 添加更多的资源信息，如主机Host、操作系统、进程信息

## 配置

```toml
[opentelemetry]
enable = false    # 运行时是否启用该插件
```

其他配置推荐使用OTEL规范中的环境变量，具体请参阅OpenTelemetry SDK文档：

* [SDK Configuration](https://opentelemetry.io/docs/languages/sdk-configuration/)
* [Environment Variable Specification](https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/)

完整代码参考[`opentelemetry-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/opentelemetry-example)

**注意**: [opentelemetry-rust](https://github.com/open-telemetry/opentelemetry-rust/issues/1678)尚未稳定，与[tracing](https://github.com/open-telemetry/opentelemetry-rust/issues/1571)的部分功能需要整合。插件会持续跟踪opentelemetry-rust和tracing的相关动态，并及时更新。