export type RoleDto = 'system' | 'user' | 'assistant';

export type MessageDto = {
  role: RoleDto,
  content: string,
  images?: string[],
}