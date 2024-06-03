use anyhow::{Context, Result};
use curl::easy::{Auth, Easy, Form};
use log::info;

pub(crate) struct Email {
    apikey: String,
    to: String,
}

impl Email {
    pub(crate) fn new(apikey: String, to: String) -> Self {
        Self { apikey, to }
    }

    pub(crate) fn send(&self, message: &str) -> Result<()> {
        let mut easy = Easy::new();
        let mut form = Form::new();
        form.part("from")
            .contents("fourbuttons@simonstjg.org".as_bytes())
            .add()
            .context("Failed to add from part")?;
        form.part("to")
            .contents(self.to.as_bytes())
            .add()
            .context("Failed to add to part")?;
        form.part("subject")
            .contents("test".as_bytes())
            .add()
            .context("Failed to add subject part")?;
        form.part("text")
            .contents(message.as_bytes())
            .add()
            .context("Failed to add text part")?;
        easy.httppost(form).context("Failed on httppost")?;
        easy.http_auth(Auth::new().basic(true))
            .context("Failed on http_auth")?;
        easy.username("api").context("Failed on username")?;
        easy.password(&self.apikey).context("Failed on password")?;
        easy.url("https://api.mailgun.net/v2/simonstjg.org/messages")
            .context("Failed on url")?;

        easy.perform().context("perform failed")?;
        let response_code = easy.response_code();

        if response_code == Ok(200) {
            info!("Sent email {} to {}", message, self.to);
        } else {
            anyhow::bail!(
                "Failed to send email {} to {}, return code was {:?}",
                message,
                self.to,
                response_code
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Email;

    #[ignore]
    #[test]
    fn send_an_email() {
        let mailgun_api_key = fs::read_to_string("./mailgun-apikey").unwrap();
        let to_address = fs::read_to_string("./to-address").unwrap();
        let email = Email::new(
            mailgun_api_key.trim().to_owned(),
            to_address.trim().to_owned(),
        );

        email.send("hello world!").unwrap();
    }
}
