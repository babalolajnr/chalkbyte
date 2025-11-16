use lettre::message::{MultiPart, SinglePart, header};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use tracing::instrument;

use crate::config::email::EmailConfig;
use crate::utils::errors::AppError;

pub struct EmailService {
    config: EmailConfig,
}

impl EmailService {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    #[instrument(skip(self))]
    pub async fn send_password_reset_email(
        &self,
        to_email: &str,
        to_name: &str,
        reset_token: &str,
    ) -> Result<(), AppError> {
        let reset_link = format!(
            "{}/reset-password?token={}",
            self.config.frontend_url, reset_token
        );

        let html_body = self.password_reset_template(to_name, &reset_link);
        let text_body = format!(
            "Hi {},\n\n\
             You requested to reset your password.\n\n\
             Click the link below to reset your password:\n\
             {}\n\n\
             This link will expire in 1 hour.\n\n\
             If you didn't request this, please ignore this email.\n\n\
             Best regards,\n\
             Chalkbyte Team",
            to_name, reset_link
        );

        self.send_email(to_email, "Password Reset Request", &text_body, &html_body)
            .await
    }

    #[instrument(skip(self))]
    pub async fn send_password_reset_confirmation(
        &self,
        to_email: &str,
        to_name: &str,
    ) -> Result<(), AppError> {
        let html_body = self.password_reset_confirmation_template(to_name);
        let text_body = format!(
            "Hi {},\n\n\
             Your password has been successfully reset.\n\n\
             If you didn't make this change, please contact support immediately.\n\n\
             Best regards,\n\
             Chalkbyte Team",
            to_name
        );

        self.send_email(
            to_email,
            "Password Reset Successful",
            &text_body,
            &html_body,
        )
        .await
    }

    #[instrument(skip(self, html_body, text_body))]
    async fn send_email(
        &self,
        to_email: &str,
        subject: &str,
        text_body: &str,
        html_body: &str,
    ) -> Result<(), AppError> {
        let from = format!("{} <{}>", self.config.from_name, self.config.from_email);

        let email = Message::builder()
            .from(
                from.parse()
                    .map_err(|e| AppError::internal_error(format!("Invalid from email: {}", e)))?,
            )
            .to(to_email
                .parse()
                .map_err(|e| AppError::internal_error(format!("Invalid to email: {}", e)))?)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN)
                            .body(text_body.to_string()),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(html_body.to_string()),
                    ),
            )
            .map_err(|e| AppError::internal_error(format!("Failed to build email: {}", e)))?;

        let mailer = if self.config.smtp_username.is_empty() {
            SmtpTransport::builder_dangerous(&self.config.smtp_host)
                .port(self.config.smtp_port)
                .build()
        } else {
            let creds = Credentials::new(
                self.config.smtp_username.clone(),
                self.config.smtp_password.clone(),
            );

            SmtpTransport::relay(&self.config.smtp_host)
                .map_err(|e| {
                    AppError::internal_error(format!("Failed to create SMTP relay: {}", e))
                })?
                .port(self.config.smtp_port)
                .credentials(creds)
                .build()
        };

        tokio::task::spawn_blocking(move || mailer.send(&email))
            .await
            .map_err(|e| AppError::internal_error(format!("Task join error: {}", e)))?
            .map_err(|e| AppError::internal_error(format!("Failed to send email: {}", e)))?;

        Ok(())
    }

    fn password_reset_template(&self, name: &str, reset_link: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Password Reset Request</title>
</head>
<body style="margin: 0; padding: 0; font-family: Arial, sans-serif; background-color: #f4f4f4;">
    <table width="100%" cellpadding="0" cellspacing="0" style="background-color: #f4f4f4; padding: 20px;">
        <tr>
            <td align="center">
                <table width="600" cellpadding="0" cellspacing="0" style="background-color: #ffffff; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1);">
                    <tr>
                        <td style="background-color: #4F46E5; padding: 30px; text-align: center;">
                            <h1 style="margin: 0; color: #ffffff; font-size: 28px;">Chalkbyte</h1>
                        </td>
                    </tr>
                    <tr>
                        <td style="padding: 40px 30px;">
                            <h2 style="margin: 0 0 20px 0; color: #333333; font-size: 24px;">Password Reset Request</h2>
                            <p style="margin: 0 0 20px 0; color: #666666; font-size: 16px; line-height: 1.5;">
                                Hi <strong>{}</strong>,
                            </p>
                            <p style="margin: 0 0 20px 0; color: #666666; font-size: 16px; line-height: 1.5;">
                                We received a request to reset your password. Click the button below to create a new password:
                            </p>
                            <table width="100%" cellpadding="0" cellspacing="0" style="margin: 30px 0;">
                                <tr>
                                    <td align="center">
                                        <a href="{}" style="display: inline-block; padding: 14px 40px; background-color: #4F46E5; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: bold;">Reset Password</a>
                                    </td>
                                </tr>
                            </table>
                            <p style="margin: 0 0 10px 0; color: #666666; font-size: 14px; line-height: 1.5;">
                                Or copy and paste this link into your browser:
                            </p>
                            <p style="margin: 0 0 20px 0; color: #4F46E5; font-size: 14px; word-break: break-all;">
                                {}
                            </p>
                            <p style="margin: 0 0 20px 0; color: #666666; font-size: 14px; line-height: 1.5;">
                                <strong>This link will expire in 1 hour.</strong>
                            </p>
                            <p style="margin: 0; color: #666666; font-size: 14px; line-height: 1.5;">
                                If you didn't request this password reset, please ignore this email or contact support if you have concerns.
                            </p>
                        </td>
                    </tr>
                    <tr>
                        <td style="background-color: #f8f9fa; padding: 20px 30px; text-align: center; border-top: 1px solid #e9ecef;">
                            <p style="margin: 0; color: #999999; font-size: 12px;">
                                This is an automated email from Chalkbyte. Please do not reply.
                            </p>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>"#,
            name, reset_link, reset_link
        )
    }

    fn password_reset_confirmation_template(&self, name: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Password Reset Successful</title>
</head>
<body style="margin: 0; padding: 0; font-family: Arial, sans-serif; background-color: #f4f4f4;">
    <table width="100%" cellpadding="0" cellspacing="0" style="background-color: #f4f4f4; padding: 20px;">
        <tr>
            <td align="center">
                <table width="600" cellpadding="0" cellspacing="0" style="background-color: #ffffff; border-radius: 8px; overflow: hidden; box-shadow: 0 2px 4px rgba(0,0,0,0.1);">
                    <tr>
                        <td style="background-color: #10B981; padding: 30px; text-align: center;">
                            <h1 style="margin: 0; color: #ffffff; font-size: 28px;">Chalkbyte</h1>
                        </td>
                    </tr>
                    <tr>
                        <td style="padding: 40px 30px;">
                            <h2 style="margin: 0 0 20px 0; color: #333333; font-size: 24px;">Password Reset Successful</h2>
                            <p style="margin: 0 0 20px 0; color: #666666; font-size: 16px; line-height: 1.5;">
                                Hi <strong>{}</strong>,
                            </p>
                            <p style="margin: 0 0 20px 0; color: #666666; font-size: 16px; line-height: 1.5;">
                                Your password has been successfully reset.
                            </p>
                            <p style="margin: 0 0 20px 0; color: #666666; font-size: 16px; line-height: 1.5;">
                                You can now log in to your account using your new password.
                            </p>
                            <div style="background-color: #FEF3C7; border-left: 4px solid #F59E0B; padding: 15px; margin: 20px 0;">
                                <p style="margin: 0; color: #92400E; font-size: 14px; line-height: 1.5;">
                                    <strong>Security Notice:</strong> If you didn't make this change, please contact support immediately.
                                </p>
                            </div>
                        </td>
                    </tr>
                    <tr>
                        <td style="background-color: #f8f9fa; padding: 20px 30px; text-align: center; border-top: 1px solid #e9ecef;">
                            <p style="margin: 0; color: #999999; font-size: 12px;">
                                This is an automated email from Chalkbyte. Please do not reply.
                            </p>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>"#,
            name
        )
    }
}
