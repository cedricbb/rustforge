use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
    response: String,
    done: bool,
}

pub struct OllamaClient {
    client: Client,
    endpoint: String,
    model: String,
}

impl OllamaClient {
    pub fn new(endpoint: String, model: String) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            model,
        }
    }

    pub async fn stream_generate(&self, prompt: &str) -> Result<impl Stream<Item = Result<String, Box<dyn std::error::Error>>>> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: true,
        };

        let response = self.client
            .post(&format!("{}/api/generate", self.endpoint))
            .json(&request)
            .send()
            .await?;

        Ok(response
            .bytes_stream()
            .map(|chunk| {
                chunk.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                    .and_then(|bytes| {
                        let text = String::from_utf8(bytes.to_vec())?;
                        if let Ok(resp) = serde_json::from_str::<OllamaResponse>(&text) {
                            Ok(resp.response)
                        } else {
                            Ok(String::new())
                        }
                    })
            }))
    }
}