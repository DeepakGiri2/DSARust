//! Transactional email: verification and password reset.
//!
//! Sending happens off the request path (`tokio::spawn` by the caller), so a
//! slow SMTP relay never slows a signup, and a failure is logged rather than
//! surfaced — the user can always ask for another link.

use crate::config::{Config, MailMode};
use anyhow::{Context, Result};
use async_trait::async_trait;

pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: String,
}

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send(&self, email: Email) -> Result<()>;
}

pub async fn from_config(cfg: &Config) -> Result<Box<dyn Mailer>> {
    Ok(match &cfg.mail {
        MailMode::Log => Box::new(LogMailer),
        MailMode::Smtp { url } => Box::new(SmtpMailer::new(url, &cfg.mail_from)?),
        MailMode::Ses => ses(cfg).await?,
    })
}

/// Development: the email is the log line. The links in it work.
pub struct LogMailer;

#[async_trait]
impl Mailer for LogMailer {
    async fn send(&self, email: Email) -> Result<()> {
        tracing::info!(to = %email.to, subject = %email.subject, "email (not sent):\n{}", email.text);
        Ok(())
    }
}

pub struct SmtpMailer {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    from: lettre::message::Mailbox,
}

impl SmtpMailer {
    fn new(url: &str, from: &str) -> Result<Self> {
        let transport = lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::from_url(url)
            .context("SMTP_URL")?
            .build();
        Ok(Self {
            transport,
            from: from.parse().context("MAIL_FROM")?,
        })
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, email: Email) -> Result<()> {
        use lettre::message::{header::ContentType, MultiPart, SinglePart};
        use lettre::AsyncTransport;
        let msg = lettre::Message::builder()
            .from(self.from.clone())
            .to(email.to.parse().context("recipient")?)
            .subject(email.subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(email.text),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(email.html),
                    ),
            )?;
        self.transport.send(msg).await?;
        Ok(())
    }
}

#[cfg(feature = "aws")]
async fn ses(cfg: &Config) -> Result<Box<dyn Mailer>> {
    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
    if let Some(region) = &cfg.aws_region {
        loader = loader.region(aws_config::Region::new(region.clone()));
    }
    let sdk = loader.load().await;
    Ok(Box::new(SesMailer {
        client: aws_sdk_sesv2::Client::new(&sdk),
        from: cfg.mail_from.clone(),
    }))
}

#[cfg(not(feature = "aws"))]
async fn ses(_cfg: &Config) -> Result<Box<dyn Mailer>> {
    anyhow::bail!("MAIL_MODE=ses needs a build with `--features aws`")
}

#[cfg(feature = "aws")]
pub struct SesMailer {
    client: aws_sdk_sesv2::Client,
    from: String,
}

#[cfg(feature = "aws")]
#[async_trait]
impl Mailer for SesMailer {
    async fn send(&self, email: Email) -> Result<()> {
        use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
        let content = |s: String| Content::builder().data(s).charset("UTF-8").build();
        let message = Message::builder()
            .subject(content(email.subject)?)
            .body(
                Body::builder()
                    .text(content(email.text)?)
                    .html(content(email.html)?)
                    .build(),
            )
            .build();
        self.client
            .send_email()
            .from_email_address(&self.from)
            .destination(Destination::builder().to_addresses(email.to).build())
            .content(EmailContent::builder().simple(message).build())
            .send()
            .await?;
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Templates
// ─────────────────────────────────────────────────────────────────────────────

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn wrap(heading: &str, body: &str, cta: &str, link: &str, footer: &str) -> String {
    format!(
        r#"<!doctype html><html><body style="margin:0;background:#0b0e14;font-family:Ubuntu,Segoe UI,Arial,sans-serif;color:#d6dbe8">
<table width="100%" cellpadding="0" cellspacing="0"><tr><td align="center" style="padding:40px 16px">
<table width="480" cellpadding="0" cellspacing="0" style="background:#11151f;border:1px solid #232a3b;border-radius:10px">
<tr><td style="padding:28px 28px 8px;font-size:20px">DSA <span style="color:#7c6cff">Visualized</span></td></tr>
<tr><td style="padding:8px 28px;font-size:16px;color:#d6dbe8">{heading}</td></tr>
<tr><td style="padding:4px 28px 18px;font-size:14px;line-height:1.6;color:#8b93a7">{body}</td></tr>
<tr><td style="padding:0 28px 24px"><a href="{link}" style="display:inline-block;background:#7c6cff;color:#fff;text-decoration:none;padding:10px 18px;border-radius:8px;font-size:14px">{cta}</a></td></tr>
<tr><td style="padding:0 28px 28px;font-size:12px;line-height:1.6;color:#6b7386">{footer}<br>If the button does not work, paste this into your browser:<br><span style="color:#22d3ee;word-break:break-all">{link}</span></td></tr>
</table></td></tr></table></body></html>"#,
        heading = escape(heading),
        body = escape(body),
        cta = escape(cta),
        link = escape(link),
        footer = escape(footer),
    )
}

pub fn verification(to: &str, name: &str, link: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: "Confirm your email for DSA Visualized".into(),
        text: format!(
            "Hi {name},\n\nConfirm this address to finish setting up your account:\n\n{link}\n\n\
             The link is valid for 48 hours. If you did not sign up, ignore this email."
        ),
        html: wrap(
            &format!("Hi {name}, confirm your email"),
            "One click and your account is fully set up — progress, playlists and runs all follow you across devices.",
            "Confirm email",
            link,
            "The link is valid for 48 hours. If you did not sign up, ignore this email.",
        ),
    }
}

pub fn password_reset(to: &str, name: &str, link: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: "Reset your DSA Visualized password".into(),
        text: format!(
            "Hi {name},\n\nSomeone (hopefully you) asked to reset your password:\n\n{link}\n\n\
             The link is valid for 1 hour and works once. If it was not you, ignore this email — \
             your password has not changed."
        ),
        html: wrap(
            &format!("Hi {name}, reset your password"),
            "Someone (hopefully you) asked to reset the password on this account.",
            "Choose a new password",
            link,
            "The link is valid for 1 hour and works once. If it was not you, ignore this email — your password has not changed.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_escape_user_text() {
        let e = verification(
            "a@b.co",
            "<script>x</script>",
            "https://x.test/verify?token=abc&y=1",
        );
        assert!(!e.html.contains("<script>x"));
        assert!(e.html.contains("&lt;script&gt;"));
        assert!(e.html.contains("token=abc&amp;y=1"));
        assert!(e.text.contains("https://x.test/verify?token=abc&y=1"));
    }
}
