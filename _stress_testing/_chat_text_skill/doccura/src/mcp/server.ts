#!/usr/bin/env bun

import 'dotenv/config';
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import { ragService } from '../core/rag-service';
import { ragChat } from '../ollama/rag-chat';
import { ollamaClient } from '../ollama/client';
import { config } from '../config';
import {
  uploadDocumentTool,
  queryRAGTool,
  listCollectionsTool,
  getStatusTool,
} from './tools';

async function main() {
  // Initialize services
  try {
    console.error('Initializing MCP RAG server...');
    await ragService.initialize();
    console.error('MCP RAG server initialized successfully');
  } catch (error) {
    console.error('Failed to initialize RAG service:', error);
    process.exit(1);
  }

  // Create MCP server
  const server = new Server(
    {
      name: 'doccura-server',
      version: '0.1.0',
    },
    {
      capabilities: {
        tools: {},
      },
    }
  );

  // List available tools
  server.setRequestHandler(ListToolsRequestSchema, async () => {
    return {
      tools: [
        {
          name: 'upload_document',
          description: 'Upload and index a PDF or TXT document into a collection',
          inputSchema: {
            type: 'object',
            properties: {
              filePath: {
                type: 'string',
                description: 'Path to the PDF or TXT file to upload',
              },
              collection: {
                type: 'string',
                description: 'Collection name to store the document',
                default: 'default',
              },
              metadata: {
                type: 'object',
                description: 'Optional metadata for the document',
              },
            },
            required: ['filePath'],
          },
        },
        {
          name: 'query_rag',
          description: 'Query the RAG system with a question and get an answer based on indexed documents',
          inputSchema: {
            type: 'object',
            properties: {
              question: {
                type: 'string',
                description: 'The question to ask',
              },
              collection: {
                type: 'string',
                description: 'Collection to search in',
                default: 'default',
              },
              limit: {
                type: 'number',
                description: 'Maximum number of results to return',
                default: 5,
              },
              threshold: {
                type: 'number',
                description: 'Similarity threshold (0-1)',
                default: 0.3,
              },
            },
            required: ['question'],
          },
        },
        {
          name: 'list_collections',
          description: 'List all available collections with their statistics',
          inputSchema: {
            type: 'object',
            properties: {},
          },
        },
        {
          name: 'get_status',
          description: 'Get system status (Ollama, embeddings, vector DB)',
          inputSchema: {
            type: 'object',
            properties: {},
          },
        },
      ],
    };
  });

  // Handle tool calls
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;

    try {
      switch (name) {
        case 'upload_document':
          return await uploadDocumentTool(args as any);

        case 'query_rag':
          return await queryRAGTool(args as any);

        case 'list_collections':
          return await listCollectionsTool();

        case 'get_status':
          return await getStatusTool();

        default:
          return {
            content: [
              {
                type: 'text',
                text: `Unknown tool: ${name}`,
              },
            ],
            isError: true,
          };
      }
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `Error: ${(error as Error).message}`,
          },
        ],
        isError: true,
      };
    }
  });

  // Start server with stdio transport
  const transport = new StdioServerTransport();
  await server.connect(transport);

  console.error('MCP RAG server running on stdio');
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});

