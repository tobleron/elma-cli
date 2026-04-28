// Core types for RAG system
export interface DocumentChunk {
  id: string;
  content: string;
  metadata: {
    source: string;
    page?: number;
    chunkIndex: number;
    totalChunks: number;
    documentType?: string;
    title?: string;
    fileSize?: number;
    fileName?: string;
  };
}

export interface SearchResult {
  content: string;
  score: number;
  metadata: DocumentChunk['metadata'];
}

export interface QueryRequest {
  question: string;
  collection: string;
  limit?: number;
  threshold?: number;
}

export interface QueryResponse {
  answer: string;
  sources: SearchResult[];
  processingTime: number;
}

export interface UploadRequest {
  collection: string;
  metadata?: Record<string, any>;
}

export interface UploadResponse {
  documentId: string;
  chunksCount: number;
  processingTime: number;
}

export interface CollectionInfo {
  name: string;
  documentCount: number;
  chunkCount: number;
  documents?: DocumentInfo[];
}

export interface DocumentInfo {
  id: string;
  fileName: string;
  title?: string;
  chunkCount: number;
  fileSize?: number;
  documentType?: string;
}

// Configuration types
export interface RagConfig {
  chunkSize: number;
  chunkOverlap: number;
  maxResults: number;
  similarityThreshold: number;
  embeddingModel: string;
  chromaPath: string;
  chromaUrl: string;
  documentsPath: string;
  embeddingsCachePath: string;
  maxFileSizeMb: number;
}

export interface OllamaConfig {
  endpoint: string;
  model: string;
  enableThinking: boolean;
}

