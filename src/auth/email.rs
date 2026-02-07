//! # Email Integration
//!
//! Email sending for authentication flows.

use std::sync::Arc;

use crate::auth::errors::{AuthError, AuthResult};

/// Email configuration
#[derive(Debug, Clone)]
pub struct EmailConfig {
    /// SMTP server host
    pub smtp_host: String,

    /// SMTP server port
    pub smtp_port: u16,

    /// SMTP username
    pub smtp_user: String,

    /// SMTP password (should come from secrets)
    pub smtp_password: String,

    /// From email address
    pub from_email: String,

    /// From name
    pub from_name: String,

    /// Base URL for links
    pub base_url: String,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            smtp_user: String::new(),
            smtp_password: String::new(),
            from_email: "noreply@aerodb.local".to_string(),
            from_name: "AeroDB".to_string(),
            base_url: "http://localhost:3000".to_string(),
        }
    }
}

/// Email template types
#[derive(Debug, Clone)]
pub enum EmailTemplate {
    /// Email verification
    Verification { token: String, user_email: String },

    /// Password reset
    PasswordReset { token: String, user_email: String },

    /// Password changed notification
    PasswordChanged { user_email: String },
}

/// Email sender trait for abstraction
pub trait EmailSender: Send + Sync {
    /// Send an email
    fn send(&self, template: EmailTemplate) -> AuthResult<()>;
}

/// Mock email sender for testing
#[derive(Debug, Default)]
pub struct MockEmailSender {
    /// Sent emails (for testing)
    pub sent: std::sync::RwLock<Vec<EmailTemplate>>,
}

impl MockEmailSender {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get number of sent emails
    pub fn sent_count(&self) -> usize {
        self.sent.read().unwrap().len()
    }

    /// Clear sent emails
    pub fn clear(&self) {
        self.sent.write().unwrap().clear();
    }
}

impl EmailSender for MockEmailSender {
    fn send(&self, template: EmailTemplate) -> AuthResult<()> {
        self.sent.write().unwrap().push(template);
        Ok(())
    }
}

/// SMTP email sender
pub struct SmtpEmailSender {
    config: EmailConfig,
}

impl SmtpEmailSender {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    fn render_template(&self, template: &EmailTemplate) -> (String, String, String) {
        match template {
            EmailTemplate::Verification { token, user_email } => {
                let subject = "Verify your email address".to_string();
                let link = format!("{}/auth/verify?token={}", self.config.base_url, token);
                let body = format!(
                    "Hello,\n\n\
                    Please verify your email address by clicking the link below:\n\n\
                    {}\n\n\
                    This link will expire in 24 hours.\n\n\
                    If you didn't create an account, you can ignore this email.\n\n\
                    Thanks,\n\
                    The AeroDB Team",
                    link
                );
                (user_email.clone(), subject, body)
            }
            EmailTemplate::PasswordReset { token, user_email } => {
                let subject = "Reset your password".to_string();
                let link = format!(
                    "{}/auth/reset-password?token={}",
                    self.config.base_url, token
                );
                let body = format!(
                    "Hello,\n\n\
                    You requested to reset your password. Click the link below:\n\n\
                    {}\n\n\
                    This link will expire in 1 hour.\n\n\
                    If you didn't request this, you can ignore this email.\n\n\
                    Thanks,\n\
                    The AeroDB Team",
                    link
                );
                (user_email.clone(), subject, body)
            }
            EmailTemplate::PasswordChanged { user_email } => {
                let subject = "Your password was changed".to_string();
                let body = format!(
                    "Hello,\n\n\
                    Your password was successfully changed.\n\n\
                    If you didn't make this change, please contact support immediately.\n\n\
                    Thanks,\n\
                    The AeroDB Team"
                );
                (user_email.clone(), subject, body)
            }
        }
    }
}

impl EmailSender for SmtpEmailSender {
    fn send(&self, template: EmailTemplate) -> AuthResult<()> {
        use lettre::{
            message::header::ContentType, transport::smtp::authentication::Credentials, Message,
            SmtpTransport, Transport,
        };

        let (to, subject, body) = self.render_template(&template);

        // Build the email message
        let email = Message::builder()
            .from(
                format!("{} <{}>", self.config.from_name, self.config.from_email)
                    .parse()
                    .map_err(|e| AuthError::EmailError(format!("Invalid from address: {}", e)))?,
            )
            .to(to
                .parse()
                .map_err(|e| AuthError::EmailError(format!("Invalid to address: {}", e)))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .map_err(|e| AuthError::EmailError(format!("Failed to build email: {}", e)))?;

        // Build SMTP transport
        let mailer = if self.config.smtp_user.is_empty() {
            // No authentication (for local development SMTP servers)
            SmtpTransport::builder_dangerous(&self.config.smtp_host)
                .port(self.config.smtp_port)
                .build()
        } else {
            // With authentication
            let creds = Credentials::new(
                self.config.smtp_user.clone(),
                self.config.smtp_password.clone(),
            );

            SmtpTransport::relay(&self.config.smtp_host)
                .map_err(|e| AuthError::EmailError(format!("SMTP relay error: {}", e)))?
                .credentials(creds)
                .port(self.config.smtp_port)
                .build()
        };

        // Send the email
        mailer
            .send(&email)
            .map_err(|e| AuthError::EmailError(format!("Failed to send email: {}", e)))?;

        Ok(())
    }
}

/// Create a boxed email sender based on config
pub fn create_email_sender(config: Option<EmailConfig>) -> Arc<dyn EmailSender> {
    match config {
        Some(cfg) => Arc::new(SmtpEmailSender::new(cfg)),
        None => Arc::new(MockEmailSender::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_email_sender() {
        let sender = MockEmailSender::new();

        sender
            .send(EmailTemplate::Verification {
                token: "test-token".to_string(),
                user_email: "test@example.com".to_string(),
            })
            .unwrap();

        assert_eq!(sender.sent_count(), 1);
    }

    #[test]
    fn test_smtp_template_rendering() {
        let config = EmailConfig::default();
        let sender = SmtpEmailSender::new(config);

        let (to, subject, body) = sender.render_template(&EmailTemplate::PasswordReset {
            token: "abc123".to_string(),
            user_email: "user@example.com".to_string(),
        });

        assert_eq!(to, "user@example.com");
        assert_eq!(subject, "Reset your password");
        assert!(body.contains("abc123"));
    }
}
