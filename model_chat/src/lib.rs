use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

#[async_trait]
pub trait ChatService {
    async fn send_message(&mut self, content: String, role: Role) -> Result<String, Box<dyn Error>>;
    async fn send_message_with_images(
        &mut self,
        message: String,
        images: Vec<String>,
        role: Role,
    ) -> Result<String, Box<dyn Error>>;
    fn set_system_message(&mut self, message: String);
    fn add_message(&mut self, content: String, role: Role);
    fn clear_history(&mut self, keep_system_message: bool);
    fn get_chat_history(&self) -> &[Message];
}

pub struct BaseChatMessage {
    system_message: Option<String>,
    messages: Vec<Message>,
    model: String,
}

impl BaseChatMessage {
    pub fn new(model: String) -> Self {
        Self {
            system_message: None,
            messages: Vec::new(),
            model,
        }
    }

    pub fn set_system_message(&mut self, message: String) {
        self.system_message = Some(message.clone());
        self.add_message(message, Role::System);
    }

    pub fn add_message(&mut self, content: String, role: Role) {
        self.messages.push(Message {
            role,
            content,
            images: None,
        });
    }

    pub fn clear_history(&mut self, keep_system_message: bool) {
        self.messages.clear();
        if keep_system_message {
            if let Some(sys_msg) = &self.system_message {
                self.add_message(sys_msg.clone(), Role::System);
            }
        }
    }

    pub fn get_chat_history(&self) -> &[Message] {
        &self.messages
    }
}

pub const OPENAI_DEFAULT_MODEL: &str = "gpt-4o-mini";

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: Message,
}

pub struct OpenAiChatService {
    base: BaseChatMessage,
    api_key: String,
    base_url: String,
}

impl OpenAiChatService {
    pub fn new(
        api_key: String,
        model: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        Self {
            base: BaseChatMessage::new(model.unwrap_or_else(|| OPENAI_DEFAULT_MODEL.to_string())),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }
}

#[async_trait]
impl ChatService for OpenAiChatService {
    async fn send_message(&mut self, content: String, role: Role) -> Result<String, Box<dyn Error>> {
        self.base.add_message(content, role);

        let client = reqwest::Client::new();
        let request = OpenAiRequest {
            model: self.base.model.clone(),
            messages: self.base.messages.clone(),
            stream: false,
        };

        let response = client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        let result: OpenAiResponse = response.json().await?;
        Ok(result.choices[0].message.content.clone())
    }

    async fn send_message_with_images(
        &mut self,
        _message: String,
        _images: Vec<String>,
        _role: Role,
    ) -> Result<String, Box<dyn Error>> {
        // TODO: Implement image support
        Ok(String::new())
    }

    fn set_system_message(&mut self, message: String) {
        self.base.set_system_message(message);
    }

    fn add_message(&mut self, content: String, role: Role) {
        self.base.add_message(content, role);
    }

    fn clear_history(&mut self, keep_system_message: bool) {
        self.base.clear_history(keep_system_message);
    }

    fn get_chat_history(&self) -> &[Message] {
        self.base.get_chat_history()
    }
}

pub const OLLAMA_DEFAULT_BASE: &str = "http://localhost:11434";
pub const OLLAMA_DEFAULT_MODEL: &str = "llama3:8b";

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<Message>,
    keep_alive: i32,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamResponse {
    message: Option<Message>,
}

pub struct OllamaChatService {
    base: BaseChatMessage,
    base_url: String,
}

impl OllamaChatService {
    pub fn new(
        model: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        Self {
            base: BaseChatMessage::new(model.unwrap_or_else(|| OLLAMA_DEFAULT_MODEL.to_string())),
            base_url: base_url.unwrap_or_else(|| OLLAMA_DEFAULT_BASE.to_string()),
        }
    }

    async fn process_stream_response(response: reqwest::Response) -> Result<String, Box<dyn Error>> {
        let mut result = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            
            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }
                
                if let Ok(response) = serde_json::from_str::<OllamaStreamResponse>(line) {
                    if let Some(message) = response.message {
                        result.push_str(&message.content);
                    }
                }
            }
        }

        Ok(result)
    }
}

#[async_trait]
impl ChatService for OllamaChatService {
    async fn send_message(&mut self, content: String, role: Role) -> Result<String, Box<dyn Error>> {
        self.base.add_message(content, role);

        let client = reqwest::Client::new();
        let request = OllamaRequest {
            model: self.base.model.clone(),
            messages: self.base.messages.clone(),
            keep_alive: 0,
        };

        let response = client
            .post(format!("{}/api/chat", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        Self::process_stream_response(response).await
    }

    async fn send_message_with_images(
        &mut self,
        message: String,
        images: Vec<String>,
        role: Role,
    ) -> Result<String, Box<dyn Error>> {
        self.add_message(message, role);
        
        // Update the last message to include images
        if let Some(last_message) = self.base.messages.last_mut() {
            last_message.images = Some(images);
        }

        let client = reqwest::Client::new();
        let request = OllamaRequest {
            model: self.base.model.clone(),
            messages: self.base.messages.clone(),
            keep_alive: 0,
        };

        let response = client
            .post(format!("{}/api/chat", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        Self::process_stream_response(response).await
    }

    fn set_system_message(&mut self, message: String) {
        self.base.set_system_message(message);
    }

    fn add_message(&mut self, content: String, role: Role) {
        self.base.add_message(content, role);
    }

    fn clear_history(&mut self, keep_system_message: bool) {
        self.base.clear_history(keep_system_message);
    }

    fn get_chat_history(&self) -> &[Message] {
        self.base.get_chat_history()
    }
}
