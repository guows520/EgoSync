export interface Mission {
  id: string;
  content: string | null;
  format: 'free' | 'structured';
  updatedAt: string;
}
