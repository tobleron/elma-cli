import { RagConfig, OllamaConfig } from '../types';

export class Config {
  private static instance: Config;
  public readonly rag: RagConfig;
  public readonly ollama: OllamaConfig;

  private constructor() {
    this.rag = {
      chunkSize: parseInt(process.env.RAG_CHUNK_SIZE || '1000'),
      chunkOverlap: parseInt(process.env.RAG_CHUNK_OVERLAP || '200'),
      maxResults: parseInt(process.env.RAG_MAX_RESULTS || '5'),
      similarityThreshold: parseFloat(process.env.RAG_SIMILARITY_THRESHOLD || '0.3'),
      embeddingModel: process.env.EMBEDDING_MODEL || 'Xenova/paraphrase-multilingual-MiniLM-L12-v2',
      chromaPath: process.env.CHROMA_PATH || './data/chroma', // Not used directly, kept for compatibility
      chromaUrl: process.env.CHROMA_URL || 'http://localhost:8000',
      documentsPath: process.env.DOCUMENTS_PATH || './data/documents',
      embeddingsCachePath: process.env.EMBEDDINGS_CACHE_PATH || './data/embeddings',
      maxFileSizeMb: parseInt(process.env.MAX_FILE_SIZE_MB || '50')
    };

    this.ollama = {
      endpoint: process.env.OLLAMA_ENDPOINT || 'http://localhost:11434',
      model: process.env.OLLAMA_MODEL || 'qwen3:1.7b',
      enableThinking: process.env.ENABLE_THINKING === 'true'
    };
  }

  public static getInstance(): Config {
    if (!Config.instance) {
      Config.instance = new Config();
    }
    return Config.instance;
  }

  public validate(): void {
    // Validate chunk settings
    if (this.rag.chunkSize <= 0) {
      throw new Error('RAG_CHUNK_SIZE must be positive');
    }
    if (this.rag.chunkOverlap >= this.rag.chunkSize) {
      throw new Error('RAG_CHUNK_OVERLAP must be less than RAG_CHUNK_SIZE');
    }

    // Validate similarity threshold
    if (this.rag.similarityThreshold < 0 || this.rag.similarityThreshold > 1) {
      throw new Error('RAG_SIMILARITY_THRESHOLD must be between 0 and 1');
    }
  }
}

export const config = Config.getInstance();

