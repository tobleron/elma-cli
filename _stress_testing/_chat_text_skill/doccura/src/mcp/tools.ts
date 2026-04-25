import * as fs from 'fs';
import * as path from 'path';
import { ragService } from '../core/rag-service';
import { ragChat } from '../ollama/rag-chat';
import { ollamaClient } from '../ollama/client';
import { embeddingService } from '../core/embeddings';
import { config } from '../config';

interface UploadDocumentArgs {
  filePath: string;
  collection?: string;
  metadata?: Record<string, any>;
}

interface QueryRAGArgs {
  question: string;
  collection?: string;
  limit?: number;
  threshold?: number;
}

export async function uploadDocumentTool(args: UploadDocumentArgs) {
  const { filePath, collection = 'default', metadata } = args;

  // Validate file exists
  if (!fs.existsSync(filePath)) {
    return {
      content: [
        {
          type: 'text',
          text: `File not found: ${filePath}`,
        },
      ],
      isError: true,
    };
  }

  // Get original filename
  const originalFilename = path.basename(filePath);

  try {
    const result = await ragService.indexDocument(
      filePath,
      collection,
      metadata,
      originalFilename
    );

    return {
      content: [
        {
          type: 'text',
          text: `Document indexed successfully!\nDocument ID: ${result.documentId}\nChunks: ${result.chunksCount}\nProcessing time: ${result.processingTime}ms`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Failed to index document: ${(error as Error).message}`,
        },
      ],
      isError: true,
    };
  }
}

export async function queryRAGTool(args: QueryRAGArgs) {
  const { question, collection = 'default', limit, threshold } = args;

  if (!question || question.trim().length === 0) {
    return {
      content: [
        {
          type: 'text',
          text: 'Question is required',
        },
      ],
      isError: true,
    };
  }

  try {
    const result = await ragChat.query({
      question,
      collection,
      limit,
      threshold,
    });

    const sourcesText = result.sources.length > 0
      ? `\n\nSources (${result.sources.length}):\n${result.sources
          .map((s, i) => `  ${i + 1}. ${s.metadata.source} (page ${s.metadata.page || 'N/A'}, score: ${s.score.toFixed(3)})`)
          .join('\n')}`
      : '';

    return {
      content: [
        {
          type: 'text',
          text: `${result.answer}${sourcesText}\n\nProcessing time: ${result.processingTime}ms`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Query failed: ${(error as Error).message}`,
        },
      ],
      isError: true,
    };
  }
}

export async function listCollectionsTool() {
  try {
    const collections = await ragService.listCollections();

    if (collections.length === 0) {
      return {
        content: [
          {
            type: 'text',
            text: 'No collections found. Use upload_document to create one.',
          },
        ],
      };
    }

    const list = collections
      .map(
        (c) =>
          `  - ${c.name}: ${c.chunkCount} chunks, ${c.documentCount} documents`
      )
      .join('\n');

    return {
      content: [
        {
          type: 'text',
          text: `Collections (${collections.length}):\n${list}`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Failed to list collections: ${(error as Error).message}`,
        },
      ],
      isError: true,
    };
  }
}

export async function getStatusTool() {
  try {
    const ollamaHealthy = await ollamaClient.checkHealth();
    const embeddingInfo = embeddingService.getModelInfo();
    const collections = await ragService.listCollections();

    const status = {
      ollama: {
        endpoint: config.ollama.endpoint,
        model: config.ollama.model,
        status: ollamaHealthy ? 'online' : 'offline',
        thinkingEnabled: config.ollama.enableThinking,
      },
      embeddings: {
        model: embeddingInfo.name,
        initialized: embeddingInfo.initialized,
      },
      vectorDb: {
        type: 'Chroma',
        path: config.rag.chromaPath,
        collections: collections.length,
      },
      rag: {
        chunkSize: config.rag.chunkSize,
        chunkOverlap: config.rag.chunkOverlap,
        maxResults: config.rag.maxResults,
        similarityThreshold: config.rag.similarityThreshold,
      },
    };

    return {
      content: [
        {
          type: 'text',
          text: JSON.stringify(status, null, 2),
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Failed to get status: ${(error as Error).message}`,
        },
      ],
      isError: true,
    };
  }
}

