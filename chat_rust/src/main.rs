use chat_rust::{ChatService, OpenAiChatService, Role};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set in .env file");

    let mut chat_service = OpenAiChatService::new(
        api_key,
        None,
        None,
    );

    chat_service.set_system_message("You are a helpful assistant.".to_string());

    let response = chat_service
        .send_message("Who are you?".to_string(), Role::User)
        .await?;

    println!("Assistant's response: {}", response);

    Ok(())
}