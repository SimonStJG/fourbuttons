use curl::easy::{Auth, Easy, Form};
use log::{error, info};

pub(crate) struct Email {
    apikey: String,
    to: String,
}

impl Email {
    pub(crate) fn new(apikey: String, to: String) -> Self {
        Self { apikey, to }
    }

    pub(crate) fn send(&self, message: &str) {
        let mut easy = Easy::new();
        let mut form = Form::new();
        form.part("from")
            .contents("fourbuttons@simonstjg.org".as_bytes())
            .add()
            .unwrap();
        form.part("to").contents(self.to.as_bytes()).add().unwrap();
        form.part("subject")
            .contents("test".as_bytes())
            .add()
            .unwrap();
        form.part("text")
            .contents(message.as_bytes())
            .add()
            .unwrap();
        easy.httppost(form).unwrap();
        easy.http_auth(Auth::new().basic(true)).unwrap();
        easy.username("api").unwrap();
        easy.password(&self.apikey).unwrap();
        easy.url("https://api.mailgun.net/v2/simonstjg.org/messages")
            .unwrap();

        match easy.perform() {
            Ok(_) => {
                let response_code = easy.response_code();
                if response_code == Ok(200) {
                    info!("Sent email {} to {}", message, self.to);
                } else {
                    error!(
                        "Failed to send email {} to {}, return code was {:?}",
                        message, self.to, response_code
                    );
                }
            }
            Err(err) => {
                error!(
                    "Failed to send email {} to {}, err was {:?}",
                    message, self.to, err
                );
            }
        }
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

        email.send("hello world!");
    }
}
