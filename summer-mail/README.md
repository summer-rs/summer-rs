
[![crates.io](https://img.shields.io/crates/v/spring-mail.svg)](https://crates.io/crates/spring-mail)
[![Documentation](https://docs.rs/spring-mail/badge.svg)](https://docs.rs/spring-mail)

## Dependencies

```toml
spring-mail = { version = "<version>" }
```

## Configuration items

```toml
[mail]
host = "smtp.gmail.com"    # SMTP mail server address,
port = 465                 # SMTP server port number
secure = true              # Enable TLS
auth = { user = "user@gmail.com", password = "passwd" } # Authentication information
test_connection = false    # Whether to test mail server connection on startup
```

## Components

After configuring the above configuration items, the plugin will automatically register a [`Mailer`](https://docs.rs/spring-mail/latest/spring_mail/type.Mailer.html)STMP asynchronous client. This object is an alias of [`lettre::AsyncSmtpTransport<Tokio1Executor>`](https://docs.rs/lettre/latest/lettre/transport/smtp/struct.AsyncSmtpTransport.html).

```rust
pub type Mailer = lettre::AsyncSmtpTransport<Tokio1Executor>;
```

## Extract the Component registered by the plugin

The `MailPlugin` plugin automatically registers an SMTP client for us. We can use `Component` to extract this connection pool from AppState. [`Component`](https://docs.rs/spring-web/latest/spring_web/extractor/struct.Component.html) is an axum [extractor](https://docs.rs/axum/latest/axum/extract/index.html).

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

For the complete code, please refer to [`mail-example`](https://github.com/spring-rs/spring-rs/tree/master/examples/mail-example)