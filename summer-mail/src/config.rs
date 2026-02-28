use schemars::JsonSchema;
use serde::Deserialize;
use summer::config::Configurable;

summer::submit_config_schema!("mail", MailerConfig);

/// SMTP mailer configuration structure.
#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "mail"]
pub struct MailerConfig {
    /// Mailer transport
    #[serde(flatten)]
    pub transport: Option<SmtpTransportConfig>,
    /// Creates a `AsyncSmtpTransportBuilder` from a [connection URL](https://docs.rs/lettre/latest/lettre/transport/smtp/struct.AsyncSmtpTransport.html#method.from_url)
    pub uri: Option<String>,
    /// Tests the SMTP connection
    #[serde(default = "bool::default")]
    pub test_connection: bool,
    /// Use stub transport. This transport logs messages and always returns the given response.
    /// It can be useful for testing purposes.
    #[serde(default = "bool::default")]
    pub stub: bool,
}

#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct SmtpTransportConfig {
    /// SMTP host. for example: localhost, smtp.gmail.com etc.
    pub host: String,
    /// SMTP port
    pub port: u16,
    /// Enable TLS
    #[serde(default = "bool::default")]
    pub secure: bool,
    /// Auth SMTP server
    pub auth: Option<MailerAuth>,
}

/// Authentication details for the mailer
#[derive(Debug, Clone, JsonSchema, Deserialize)]
pub struct MailerAuth {
    /// User
    pub user: String,
    /// Password
    pub password: String,
}
