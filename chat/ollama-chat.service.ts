import { BaseChatMessage } from "./base-chat-service.ts";
import { IChatService } from "./chat-service.interface.ts";
import { RoleDto } from "./chat.dto.ts";

export const OLLAMA_DEFAULT_BASE = "http://localhost:11434";
export const OLLAMA_DEFAULT_MODEL = "llama3.2:3b";

export class OllamaChatService extends BaseChatMessage implements IChatService {
  constructor(private baseUrl: string = OLLAMA_DEFAULT_BASE, protected model: string = OLLAMA_DEFAULT_MODEL) {
    super(model);
  }

  async sendMessage(content: string, role: RoleDto): Promise<string> {
    this.addMessage(content, role);
    
    const response = await fetch(`${this.baseUrl}/api/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: this.model,
        keep_alive: 0,
        messages: this.messages,
      }),
    });

    return await this.processResponse(response);
  }

  async sendMessageWithImages(message: string, images: Array<string>, role: RoleDto): Promise<string> {
    this.messages.push(
      {
        role,
        content: message,
        images,
      },
    );

    const response = await fetch(`${this.baseUrl}/api/chat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model: this.model,
        keep_alive: 0,
        messages: this.messages,
      })
    });

    return await this.processResponse(response);
  }

  async processResponse(response: Response): Promise<string> {
    const reader = response.body?.getReader();

    if (!reader) {
      throw new Error("No response body");
    }
    let result = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      const text = new TextDecoder().decode(value);
      const lines = text.split('\n').filter(line => line.trim());

      for (const line of lines) {
        try {
          const data = JSON.parse(line);
          if (data.message?.content) {
            result += data.message.content;
          }
        } catch (e) {
          console.error('Error parsing JSON:', line);
        }
      }
    }

    return result;
  }
}