use std::process::Command;
use anyhow::Result;
use chat_rust::{ChatService, OpenAiChatService, Role};
use dotenv::dotenv;
use tokio;

#[derive(Debug)]
pub enum ToolResult {
    Success(String),
    Error(String),
}

pub trait Tool: Send {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn run(&self, args: Vec<String>) -> Result<ToolResult>;
}

#[derive(Debug, Clone)]
pub struct SystemCommandTool {
    name: String,
    description: String,
    command: String,
}

impl SystemCommandTool {
    pub fn new(name: String, description: String, command: String) -> Self {
        Self {
            name,
            description,
            command,
        }
    }
}

impl Tool for SystemCommandTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn run(&self, args: Vec<String>) -> Result<ToolResult> {
        let output = Command::new(&self.command)
            .args(&args)
            .output()?;

        if output.status.success() {
            Ok(ToolResult::Success(String::from_utf8_lossy(&output.stdout).to_string()))
        } else {
            Ok(ToolResult::Error(String::from_utf8_lossy(&output.stderr).to_string()))
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatTool {
    name: String,
    description: String,
}

impl ChatTool {
    pub fn new(name: String, description: String) -> Self {
        Self { name, description }
    }
}

impl Tool for ChatTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn run(&self, args: Vec<String>) -> Result<ToolResult> {
        let rt = tokio::runtime::Runtime::new()?;
        
        rt.block_on(async {
            dotenv().ok();
            
            let api_key = match std::env::var("OPENAI_API_KEY") {
                Ok(key) => key,
                Err(_) => return Ok(ToolResult::Error("OPENAI_API_KEY not found in .env file".to_string())),
            };

            let mut chat_service = OpenAiChatService::new(api_key, None, None);
            
            chat_service.set_system_message(
                "You are a cybersecurity expert. 
                 Your focus is reconnaissance.
                 You will receive an Nmap XML report.
                 Your task is to analyze the report and provide a summary of the findings.
                 The summary will be concise and to the point. 
                 The summary will me in markdown format.
                 Bullet points are preferred."
                    .to_string()
            );

            let message = args.join(" ");
            match chat_service.send_message(message, Role::User).await {
                Ok(response) => Ok(ToolResult::Success(response)),
                Err(e) => Ok(ToolResult::Error(e.to_string())),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_command_tool() {
        let tool = SystemCommandTool::new(
            "Echo".to_string(),
            "Echo command".to_string(),
            "echo".to_string(),
        );

        let args = vec!["hello".to_string()];
        let result = tool.run(args).unwrap();

        match result {
            ToolResult::Success(output) => assert!(output.contains("hello")),
            ToolResult::Error(_) => panic!("Expected success"),
        }
    }
}