import { config } from '../config';

export interface OllamaMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface OllamaChatRequest {
  model: string;
  messages: OllamaMessage[];
  stream?: boolean;
  options?: {
    temperature?: number;
    top_p?: number;
    top_k?: number;
    num_predict?: number;
    think?: boolean; // Control thinking for Qwen models
  };
}

export interface OllamaChatResponse {
  model: string;
  created_at: string;
  message: {
    role: string;
    content: string;
  };
  done: boolean;
  total_duration?: number;
  load_duration?: number;
  prompt_eval_count?: number;
  prompt_eval_duration?: number;
  eval_count?: number;
  eval_duration?: number;
}

export interface OllamaStreamChunk {
  model: string;
  created_at: string;
  message?: {
    role: string;
    content: string;
  };
  done: boolean;
  total_duration?: number;
  load_duration?: number;
  prompt_eval_count?: number;
  prompt_eval_duration?: number;
  eval_count?: number;
  eval_duration?: number;
}

export class OllamaClient {
  private endpoint: string;
  private model: string;
  private enableThinking: boolean;

  constructor() {
    this.endpoint = config.ollama.endpoint;
    this.model = config.ollama.model;
    this.enableThinking = config.ollama.enableThinking;
  }

  /**
   * Check if Ollama is available
   */
  async checkHealth(): Promise<boolean> {
    try {
      const response = await fetch(`${this.endpoint}/api/tags`);
      return response.ok;
    } catch (error) {
      return false;
    }
  }

  /**
   * Chat with Ollama (non-streaming)
   */
  async chat(messages: OllamaMessage[], options?: { temperature?: number }): Promise<string> {
    const request: OllamaChatRequest = {
      model: this.model,
      messages,
      stream: false,
      options: {
        temperature: options?.temperature || 0.7,
        think: this.enableThinking // Control thinking based on config
      }
    };

    try {
      const response = await fetch(`${this.endpoint}/api/chat`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(request)
      });

      if (!response.ok) {
        throw new Error(`Ollama API error: ${response.status} ${response.statusText}`);
      }

      const data: OllamaChatResponse = await response.json();
      return data.message.content;

    } catch (error) {
      console.error('Ollama chat failed', error);
      throw new Error(`Failed to chat with Ollama: ${(error as Error).message}`);
    }
  }

  /**
   * Chat with Ollama (streaming)
   */
  async *chatStream(messages: OllamaMessage[], options?: { temperature?: number }): AsyncGenerator<string, void, unknown> {
    const request: OllamaChatRequest = {
      model: this.model,
      messages,
      stream: true,
      options: {
        temperature: options?.temperature || 0.7,
        think: this.enableThinking // Control thinking based on config
      }
    };

    try {
      const response = await fetch(`${this.endpoint}/api/chat`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(request)
      });

      if (!response.ok) {
        throw new Error(`Ollama API error: ${response.status} ${response.statusText}`);
      }

      if (!response.body) {
        throw new Error('Response body is null');
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();

        if (done) {
          break;
        }

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || ''; // Keep incomplete line in buffer

        for (const line of lines) {
          if (line.trim()) {
            try {
              const chunk: OllamaStreamChunk = JSON.parse(line);
              
              if (chunk.message?.content) {
                yield chunk.message.content;
              }

              if (chunk.done) {
                return;
              }
            } catch (error) {
              // Skip invalid JSON lines
              console.warn('Failed to parse Ollama stream chunk', error);
            }
          }
        }
      }

    } catch (error) {
      console.error('Ollama chat stream failed', error);
      throw new Error(`Failed to stream chat with Ollama: ${(error as Error).message}`);
    }
  }

  /**
   * Get available models
   */
  async listModels(): Promise<string[]> {
    try {
      const response = await fetch(`${this.endpoint}/api/tags`);
      
      if (!response.ok) {
        throw new Error(`Ollama API error: ${response.status} ${response.statusText}`);
      }

      const data = await response.json();
      return data.models?.map((m: any) => m.name) || [];

    } catch (error) {
      console.error('Failed to list Ollama models', error);
      return [];
    }
  }

  /**
   * Get current model
   */
  getModel(): string {
    return this.model;
  }

  /**
   * Set model
   */
  setModel(model: string): void {
    this.model = model;
  }
}

// Singleton instance
export const ollamaClient = new OllamaClient();

