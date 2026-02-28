
[![crates.io](https://img.shields.io/crates/v/spring-mail.svg)](https://crates.io/crates/spring-mail)
[![Documentation](https://docs.rs/spring-mail/badge.svg)](https://docs.rs/spring-mail)

## 依赖

```toml
spring-mail = { version = "<version>" }
```

## 配置项

```toml
[mail]
host = "smtp.gmail.com"                                 # SMTP邮件服务器地址，
port = 465                                              # SMTP服务器端口号
secure = true                                           # 开启TLS加密
auth = { user = "user@gmail.com", password = "passwd" } # 认证信息
test_connection = false                                 # 启动时是否测试邮件服务器连接
```

## 组件

配置完上述配置项后，插件会自动注册一个[`Mailer`](https://docs.rs/spring-mail/latest/spring_mail/type.Mailer.html)STMP异步客户端。该对象是[`lettre::AsyncSmtpTransport<Tokio1Executor>`](https://docs.rs/lettre/latest/lettre/transport/smtp/struct.AsyncSmtpTransport.html)的别名。

```rust
pub type Mailer = lettre::AsyncSmtpTransport<Tokio1Executor>;
```

## 提取插件注册的Component

`MailPlugin`插件为我们自动注册了一个SMTP客户端，我们可以使用`Component`从AppState中提取这个连接池，[`Component`](https://docs.rs/spring-web/latest/spring_web/extractor/struct.Component.html)是一个axum的[extractor](https://docs.rs/axum/latest/axum/extract/index.html)。

```rust
async fn send_mail(Component(mailer): Component<Mailer>) -> Result<impl IntoResponse> {
    let email = Message::builder()
        .from("NoBody <nobody@domain.tld>".parse().unwrap())
        .reply_to("Yuin <yuin@domain.tld>".parse().unwrap())
        .to("hff1996723@163.com".parse().unwrap())
        .subject("Happy new year")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from("Be happy!"))
        .unwrap();
    let resp = mailer.send(email).await.context("send mail failed")?;
    Ok(Json(resp))
}
```

完整代码参考[`mail-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/mail-example)