use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::DynamicImage;
use reqwest::Client;
use std::io::Cursor;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum TranslateError {
    #[error("request cancelled")]
    Cancelled,
    #[error("image encoding failed: {0}")]
    ImageEncode(#[from] image::ImageError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected api response")]
    BadResponse,
}

pub struct TranslateEngine {
    client: Client,
    gateway_url: String,
    model: String,
    api_key: String,
}

impl TranslateEngine {
    pub fn new(gateway_url: String, model: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            gateway_url,
            model,
            api_key,
        }
    }

    pub async fn translate(
        &self,
        image: &DynamicImage,
        target_language: &str,
        cancel: CancellationToken,
    ) -> Result<String, TranslateError> {
        if cancel.is_cancelled() {
            return Err(TranslateError::Cancelled);
        }

        let mut buf = Cursor::new(Vec::new());
        image.write_to(&mut buf, image::ImageFormat::Jpeg)?;
        let b64 = BASE64.encode(buf.get_ref());

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": { "url": format!("data:image/jpeg;base64,{}", b64) }
                    },
                    {
                        "type": "text",
                        "text": format!(
                            "You are a translation assistant. Extract all text visible in this image and translate it to {}. Return ONLY the translated text, preserving paragraph breaks. If no text is visible, return empty string.",
                            target_language
                        )
                    }
                ]
            }],
            "max_tokens": 1024
        });

        let request = self
            .client
            .post(format!("{}/chat/completions", self.gateway_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send();

        let response = tokio::select! {
            res = request => res.map_err(TranslateError::Http)?,
            _ = cancel.cancelled() => return Err(TranslateError::Cancelled),
        };

        let json: serde_json::Value = response.json().await?;
        let text = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or(TranslateError::BadResponse)?
            .to_string();

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbImage};
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn translate_returns_text_from_api() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{ "message": { "content": "Hello world" } }]
            })))
            .mount(&server)
            .await;

        let engine = TranslateEngine::new(
            server.uri(),
            "gpt-5.4-nano".to_string(),
            "test-key".to_string(),
        );
        let img = DynamicImage::ImageRgb8(RgbImage::new(10, 10));
        let result = engine.translate(&img, "English", CancellationToken::new()).await;
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[tokio::test]
    async fn translate_returns_cancelled_when_token_is_pre_cancelled() {
        let server = MockServer::start().await;
        let engine = TranslateEngine::new(
            server.uri(),
            "gpt-5.4-nano".to_string(),
            "test-key".to_string(),
        );
        let img = DynamicImage::ImageRgb8(RgbImage::new(10, 10));
        let token = CancellationToken::new();
        token.cancel();
        let result = engine.translate(&img, "English", token).await;
        assert!(matches!(result, Err(TranslateError::Cancelled)));
    }
}
