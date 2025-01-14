import { MessageDto, RoleDto } from "./chat.dto.ts";

export interface IChatService {
  setSystemMessage(message: string): void;
  addMessage(content: string, role: RoleDto): void;
  sendMessage(content: string, role: RoleDto): Promise<string>;
  sendMessageWithImages(message: string, images: Array<string>, role: RoleDto): Promise<string>;
  clearHistory(keepSystemMessage?: boolean): void;
  getChatHistory(): MessageDto[];
}
