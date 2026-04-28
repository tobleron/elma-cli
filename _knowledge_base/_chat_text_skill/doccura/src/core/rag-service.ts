import * as path from 'path';
import * as fs from 'fs';
import { v4 as uuidv4 } from 'uuid';
import { config } from '../config';
import { PDFProcessor } from './pdf-processor';
import { TextChunker } from './chunker';
import { LocalEmbeddingService } from './embeddings';
import { ChromaVectorDB } from './vector-db';
import {
  DocumentChunk,
  QueryRequest,
  QueryResponse,
  UploadResponse,
  CollectionInfo
} from '../types';

export class RAGService {
  private pdfProcessor: PDFProcessor;
  private chunker: TextChunker;
  private embeddings: LocalEmbeddingService;
  private vectorDb: ChromaVectorDB;
  private initialized = false;

  constructor() {
    this.pdfProcessor = new PDFProcessor();
    this.chunker = new TextChunker();
    this.embeddings = new LocalEmbeddingService();
    this.vectorDb = new ChromaVectorDB();
  }

  /**
   * Initialize the RAG service
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    try {
      // Initialize embeddings
      process.stdout.write('Initializing embedding model... ');
      await this.embeddings.initialize();
      console.log('✓');
      
      // Initialize vector DB (message handled in index.tsx)
      await this.vectorDb.initialize();
      
      // Ensure directories exist
      this.ensureDirectory(config.rag.documentsPath);
      this.ensureDirectory(config.rag.embeddingsCachePath);

      this.initialized = true;

    } catch (error) {
      console.error('Failed to initialize RAG service', error);
      throw new Error(`RAG service initialization failed: ${(error as Error).message}`);
    }
  }

  /**
   * Ensure database is initialized
   */
  private async ensureInitialized(): Promise<void> {
    if (!this.initialized) {
      await this.initialize();
    }
  }

  /**
   * Ensure directory exists
   */
  private ensureDirectory(dirPath: string): void {
    if (!fs.existsSync(dirPath)) {
      fs.mkdirSync(dirPath, { recursive: true });
    }
  }

  /**
   * Index a document (PDF or TXT)
   */
  async indexDocument(
    filePath: string,
    collectionName: string,
    metadata?: Record<string, any>,
    originalFilename?: string
  ): Promise<UploadResponse> {
    await this.ensureInitialized();

    const startTime = Date.now();
    const documentId = uuidv4();
    const isPdf = filePath.toLowerCase().endsWith('.pdf');

    try {
      console.log(`Indexing document: ${filePath} into collection: ${collectionName}`);

      // Validate document file
      if (!this.pdfProcessor.validateFile(filePath, originalFilename)) {
        throw new Error('Invalid document file');
      }

      // Extract text
      const text = await this.pdfProcessor.extractText(filePath, originalFilename);

      if (!text.trim()) {
        throw new Error('No text could be extracted from document');
      }

      // Get document metadata
      let docMetadata = {};
      if (isPdf) {
        docMetadata = await this.pdfProcessor.getMetadata(filePath) || {};
      } else {
        // For text files, create basic metadata
        const stats = fs.statSync(filePath);
        docMetadata = {
          pages: 1,
          title: path.basename(filePath, '.txt'),
          fileSize: stats.size,
          fileName: path.basename(filePath)
        };
      }

      // Chunk the text
      const textChunks = this.chunker.chunkText(text);
      console.log(`Created ${textChunks.length} chunks`);

      // Create document chunks with metadata
      const chunks: DocumentChunk[] = textChunks.map((content, index) => ({
        id: `${documentId}_chunk_${index}`,
        content,
        metadata: {
          source: path.basename(filePath),
          page: isPdf ? Math.floor(index / 10) + 1 : 1, // Rough page estimation for PDFs
          chunkIndex: index,
          totalChunks: textChunks.length,
          documentType: isPdf ? 'pdf' : 'txt',
          ...metadata,
          ...docMetadata
        }
      }));

      // Generate embeddings
      const embeddings = await this.embeddings.generateEmbeddings(
        chunks.map(chunk => chunk.content)
      );

      // Store in vector database
      await this.vectorDb.addDocuments(collectionName, chunks, embeddings);

      // Copy file to documents directory for persistence
      const extension = isPdf ? '.pdf' : '.txt';
      const destPath = path.join(config.rag.documentsPath, collectionName, `${documentId}${extension}`);
      this.ensureDirectory(path.dirname(destPath));
      fs.copyFileSync(filePath, destPath);

      const processingTime = Date.now() - startTime;

      console.log(`Document indexed successfully in ${processingTime}ms`, {
        documentId,
        collection: collectionName,
        chunks: chunks.length
      });

      return {
        documentId,
        chunksCount: chunks.length,
        processingTime
      };

    } catch (error) {
      console.error(`Document indexing failed: ${filePath}`, error);
      throw new Error(`Indexing failed: ${(error as Error).message}`);
    }
  }

  /**
   * Query the RAG system (returns context only, no LLM)
   */
  async query(request: QueryRequest): Promise<QueryResponse> {
    await this.ensureInitialized();

    const startTime = Date.now();

    try {
      const { question, collection, limit, threshold } = request;

      console.log(`Processing query: "${question}" in collection: ${collection}`);

      // Generate query embedding
      const queryEmbedding = await this.embeddings.generateQueryEmbedding(question);

      // Search vector database
      const searchResults = await this.vectorDb.search(
        collection,
        queryEmbedding,
        limit || config.rag.maxResults,
        threshold || config.rag.similarityThreshold
      );

      const processingTime = Date.now() - startTime;

      return {
        answer: '', // Will be filled by RAG chat
        sources: searchResults,
        processingTime
      };

    } catch (error) {
      console.error('Query failed', error);
      throw new Error(`Query failed: ${(error as Error).message}`);
    }
  }

  /**
   * List all collections
   */
  async listCollections(includeDocuments: boolean = false): Promise<CollectionInfo[]> {
    await this.ensureInitialized();

    try {
      const collectionNames = await this.vectorDb.listCollections();
      const collections: CollectionInfo[] = [];

      for (const name of collectionNames) {
        if (!name || name === 'undefined') {
          console.warn('Skipping collection with invalid name:', name);
          continue;
        }
        
        try {
          const stats = await this.vectorDb.getCollectionStats(name);
          if (stats) {
            const documents = includeDocuments 
              ? await this.vectorDb.getCollectionDocuments(name)
              : undefined;

            collections.push({
              name: stats.name,
              documentCount: documents?.length || 0,
              chunkCount: stats.count,
              documents
            });
          }
        } catch (statsError) {
          console.warn(`Failed to get stats for collection ${name}:`, statsError);
          // Still add collection with 0 chunks
          collections.push({
            name,
            documentCount: 0,
            chunkCount: 0
          });
        }
      }

      return collections;

    } catch (error) {
      console.error('Failed to list collections', error);
      return [];
    }
  }

  /**
   * Get documents in a collection
   */
  async getCollectionDocuments(collectionName: string) {
    await this.ensureInitialized();
    return await this.vectorDb.getCollectionDocuments(collectionName);
  }

  /**
   * Delete documents from a collection
   */
  async deleteDocuments(collectionName: string, documentIds: string[]): Promise<void> {
    await this.ensureInitialized();
    await this.vectorDb.deleteDocuments(collectionName, documentIds);
    
    // Also delete from documents directory
    const documentsPath = path.join(config.rag.documentsPath, collectionName);
    // Note: We don't delete files here as we don't track which file belongs to which documentId
    // This is a limitation - we'd need to store this mapping
  }

  /**
   * Delete a collection
   */
  async deleteCollection(name: string): Promise<void> {
    await this.ensureInitialized();

    try {
      await this.vectorDb.deleteCollection(name);
      
      // Also delete documents directory
      const documentsPath = path.join(config.rag.documentsPath, name);
      if (fs.existsSync(documentsPath)) {
        fs.rmSync(documentsPath, { recursive: true, force: true });
      }

      console.log(`Deleted collection: ${name}`);

    } catch (error) {
      console.error(`Failed to delete collection: ${name}`, error);
      throw new Error(`Delete collection failed: ${(error as Error).message}`);
    }
  }
}

// Singleton instance
export const ragService = new RAGService();

