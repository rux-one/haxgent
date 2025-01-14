import { MessageDto, RoleDto } from "./chat.dto.ts";

export abstract class BaseChatMessage {
  protected systemMessage: string;
  protected messages: MessageDto[] = [];
  protected model: string;

  constructor(model: string) {
    this.model = model;
  }

  addMessage(content: string, role: RoleDto): void {
    this.messages.push({
      content,
      role,
    });
  } 

  async setModel(model: string): Promise<void> {
    this.model = model;
  }

  setSystemMessage(message: string): void {
    this.systemMessage = message;

    this.addMessage(this.systemMessage, 'system');
  } 

  clearHistory(keepSystemMessage: boolean = true): void {
    this.messages = [];
    if (keepSystemMessage) {
      this.addMessage(this.systemMessage, 'system');
    }
  }

  getChatHistory(): MessageDto[] {
    return this.messages;
  }
}