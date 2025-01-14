import { BaseChatMessage } from "./base-chat-service.ts";
import { IChatService } from "./chat-service.interface.ts";
import { RoleDto } from "./chat.dto.ts";

export const OPENAI_DEFAULT_MODEL = "gpt-3.5-turbo";

export class OpenAiChatService extends BaseChatMessage implements IChatService {
  constructor(private apiKey: string, protected model: string = OPENAI_DEFAULT_MODEL, private baseUrl: string = "https://api.openai.com/v1") {
    super(model);
  }

  async sendMessage(content: string, role: RoleDto): Promise<string> {
    this.addMessage(content, role);
    
    const response = await fetch(`${this.baseUrl}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`,
      },
      body: JSON.stringify({
        model: this.model,
        messages: this.messages,
        stream: false,
      }),
    });

    const result = await response.json();
    return result.choices[0].message.content;
  }

  async sendMessageWithImages(message: string, images: Array<string>, role: RoleDto): Promise<string> {
    return '';
    // return await this.processResponse(response);
  }
}
