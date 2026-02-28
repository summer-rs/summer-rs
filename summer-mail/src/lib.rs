//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-mail)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod config;

pub use lettre::message::*;
pub use lettre::transport::smtp::response::Response;
pub use lettre::AsyncTransport;
pub use lettre::Message;

use anyhow::Context;
use config::MailerConfig;
use config::SmtpTransportConfig;
use lettre::address::Envelope;
use lettre::transport::smtp::response::Category;
use lettre::transport::smtp::response::Code;
use lettre::transport::smtp::response::Detail;
use lettre::transport::smtp::response::Severity;
use lettre::{transport::smtp::authentication::Credentials, Tokio1Executor};
use summer::async_trait;
use summer::config::ConfigRegistry;
use summer::plugin::MutableComponentRegistry;
use summer::{app::AppBuilder, error::Result, plugin::Plugin};

pub type TokioMailerTransport = lettre::AsyncSmtpTransport<Tokio1Executor>;
pub type StubMailerTransport = lettre::transport::stub::AsyncStubTransport;

#[derive(Clone)]
pub enum Mailer {
    Tokio(TokioMailerTransport),
    Stub(StubMailerTransport),
}

#[async_trait]
impl AsyncTransport for Mailer {
    type Ok = Response;
    type Error = summer::error::AppError;

    async fn send_raw(&self, envelope: &Envelope, email: &[u8]) -> Result<Self::Ok> {
        Ok(match self {
            Self::Tokio(tokio_transport) => tokio_transport
                .send_raw(envelope, email)
                .await
                .context("mailer send failed")?,
            Self::Stub(stub_transport) => {
                stub_transport
                    .send_raw(envelope, email)
                    .await
                    .context("stub mailer send failed")?;
                Response::new(
                    Code {
                        severity: Severity::PositiveCompletion,
                        category: Category::MailSystem,
                        detail: Detail::Zero,
                    },
                    vec!["stub mailer send success".to_string()],
                )
            }
        })
    }
}

pub struct MailPlugin;

#[async_trait]
impl Plugin for MailPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<MailerConfig>()
            .expect("mail plugin config load failed");

        let mailer = if config.stub {
            Mailer::Stub(StubMailerTransport::new_ok())
        } else {
            let sender = if let Some(uri) = config.uri {
                TokioMailerTransport::from_url(&uri)
                    .expect("build mail plugin failed")
                    .build()
            } else if let Some(transport) = config.transport {
                Self::build_smtp_transport(&transport).expect("build mail plugin failed")
            } else {
                panic!("The mail plugin is missing necessary smtp transport configuration");
            };
            if config.test_connection
                && !sender
                    .test_connection()
                    .await
                    .expect("test mail connection failed")
            {
                panic!("Unable to connect to the mail server");
            }
            Mailer::Tokio(sender)
        };

        app.add_component(mailer);
    }
}

impl MailPlugin {
    fn build_smtp_transport(config: &SmtpTransportConfig) -> Result<TokioMailerTransport> {
        let mut transport_builder = if config.secure {
            TokioMailerTransport::relay(&config.host)
                .with_context(|| format!("build mailer failed: {}", config.host))?
                .port(config.port)
        } else {
            TokioMailerTransport::builder_dangerous(&config.host).port(config.port)
        };

        if let Some(auth) = config.auth.as_ref() {
            let credentials = Credentials::new(auth.user.clone(), auth.password.clone());
            transport_builder = transport_builder.credentials(credentials);
        }

        Ok(transport_builder.build())
    }
}
