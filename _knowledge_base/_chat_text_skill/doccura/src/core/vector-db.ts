import { ChromaClient } from 'chromadb';
import * as fs from 'fs';
import * as path from 'path';
import { config } from '../config';
import { DocumentChunk, SearchResult } from '../types';

export class ChromaVectorDB {
  private client: ChromaClient;
  private initialized = false;
  private collections: Map<string, any> = new Map();

  constructor() {
    // Chroma requires a server - use local server or default to localhost:8000
    // For embedded mode, you need to run Chroma server separately
    const chromaUrl = process.env.CHROMA_URL || 'http://localhost:8000';
    
    // Initialize Chroma client pointing to server
    this.client = new ChromaClient({
      path: chromaUrl
    });
  }

  /**
   * Initialize the database
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    try {
      // Test connection
      await this.client.heartbeat();
      
      this.initialized = true;

      // Load existing collections
      await this.loadCollections();

    } catch (error) {
      console.error('Failed to initialize Chroma Vector DB', error);
      throw new Error(`Vector DB initialization failed: ${(error as Error).message}`);
    }
  }

  /**
   * Load existing collections
   */
  private async loadCollections(): Promise<void> {
    try {
      // listCollections() returns string[] (just names)
      const collectionNames = await this.client.listCollections();
      
      // Clear existing collections cache
      this.collections.clear();
      
      // Load each collection object
      for (const collectionName of collectionNames) {
        // Skip invalid names
        if (!collectionName || collectionName === 'undefined' || !collectionName.trim()) {
          continue;
        }
        
        try {
          // Get the collection object
          const collectionObj = await this.client.getCollection({ name: collectionName });
          this.collections.set(collectionName, collectionObj);
        } catch (e) {
          console.warn(`Could not load collection "${collectionName}":`, e);
        }
      }
    } catch (error) {
      console.warn('Failed to load collections', error);
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
   * Get or create a collection
   */
  private async getOrCreateCollection(collectionName: string) {
    await this.ensureInitialized();

    // First try to get existing collection
    try {
      const existingCollection = await this.client.getCollection({ name: collectionName });
      if (existingCollection) {
        this.collections.set(collectionName, existingCollection);
        return existingCollection;
      }
    } catch (e) {
      // Collection doesn't exist, will create it
    }

    // If not in cache, try to get it
    if (this.collections.has(collectionName)) {
      return this.collections.get(collectionName);
    }

    // Create new collection
    const collection = await this.client.createCollection({
      name: collectionName,
      metadata: { description: `Collection for ${collectionName}` }
    });

    this.collections.set(collectionName, collection);
    console.log(`Created collection: ${collectionName}`);
    
    return collection;
  }

  /**
   * Add documents to a collection
   */
  async addDocuments(
    collectionName: string,
    chunks: DocumentChunk[],
    embeddings: number[][]
  ): Promise<void> {
    await this.ensureInitialized();

    try {
      const collection = await this.getOrCreateCollection(collectionName);

      // Prepare data for Chroma
      const ids = chunks.map(chunk => chunk.id);
      const documents = chunks.map(chunk => chunk.content);
      const metadatas = chunks.map(chunk => ({
        source: chunk.metadata.source,
        page: chunk.metadata.page || 0,
        chunkIndex: chunk.metadata.chunkIndex,
        totalChunks: chunk.metadata.totalChunks,
        documentType: chunk.metadata.documentType || 'unknown',
        title: chunk.metadata.title || '',
        fileSize: chunk.metadata.fileSize || 0,
        fileName: chunk.metadata.fileName || ''
      }));

      // Add to Chroma
      await collection.add({
        ids,
        embeddings,
        documents,
        metadatas
      });

      console.log(`Added ${chunks.length} documents to collection: ${collectionName}`);

    } catch (error) {
      console.error(`Failed to add documents to collection: ${collectionName}`, error);
      throw new Error(`Add documents failed: ${(error as Error).message}`);
    }
  }

  /**
   * Search for similar documents
   */
  async search(
    collectionName: string,
    queryEmbedding: number[],
    limit: number = config.rag.maxResults,
    threshold: number = config.rag.similarityThreshold
  ): Promise<SearchResult[]> {
    await this.ensureInitialized();

    try {
      if (!this.collections.has(collectionName)) {
        return [];
      }

      const collection = this.collections.get(collectionName);

      // Query Chroma
      const results = await collection.query({
        queryEmbeddings: [queryEmbedding],
        nResults: limit * 2, // Get more results to filter by threshold
        include: ['documents', 'metadatas', 'distances']
      });

      if (!results.ids || results.ids.length === 0 || !results.ids[0]) {
        return [];
      }

      // Convert Chroma results to SearchResult format
      // Chroma returns distances (lower is better), we need similarity scores (higher is better)
      const searchResults: SearchResult[] = [];
      
      const ids = results.ids[0] || [];
      const documents = results.documents?.[0] || [];
      const metadatas = results.metadatas?.[0] || [];
      const distances = results.distances?.[0] || [];

      for (let i = 0; i < ids.length; i++) {
        // Convert distance to similarity score (1 - normalized distance)
        // Chroma uses cosine distance, so similarity = 1 - distance
        const distance = distances[i] || 0;
        const similarity = 1 - distance;

        // Filter by threshold (but log for debugging)
        if (similarity >= threshold) {
          searchResults.push({
            content: documents[i] || '',
            score: similarity,
            metadata: {
              source: metadatas[i]?.source || '',
              page: metadatas[i]?.page || 0,
              chunkIndex: metadatas[i]?.chunkIndex || 0,
              totalChunks: metadatas[i]?.totalChunks || 0,
              documentType: metadatas[i]?.documentType,
              title: metadatas[i]?.title,
              fileSize: metadatas[i]?.fileSize,
              fileName: metadatas[i]?.fileName
            }
          });
        }
      }

      // Log raw results for debugging
      if (ids.length > 0) {
        const topDistances = distances.slice(0, 5).map(d => d.toFixed(4));
        const topSimilarities = distances.slice(0, 5).map(d => (1 - d).toFixed(4));
        console.log(`Raw Chroma results: ${ids.length} total, top 5 distances: [${topDistances.join(', ')}], similarities: [${topSimilarities.join(', ')}]`);
      }

      // Log search results for debugging
      console.log(`Search results: ${searchResults.length} found (threshold: ${threshold}), top scores:`, 
        searchResults.slice(0, 3).map(r => r.score.toFixed(3)));

      // If no results with threshold, include all results (for very low thresholds)
      if (searchResults.length === 0 && threshold <= 0.1) {
        console.log(`No results with threshold ${threshold}, including all results...`);
        for (let i = 0; i < Math.min(ids.length, limit * 10); i++) {
          const distance = distances[i] || 0;
          const similarity = 1 - distance;
          
          // Include all results if threshold is very low or 0
          if (threshold === 0 || similarity >= threshold) {
            searchResults.push({
              content: documents[i] || '',
              score: similarity,
              metadata: {
                source: metadatas[i]?.source || '',
                page: metadatas[i]?.page || 0,
                chunkIndex: metadatas[i]?.chunkIndex || 0,
                totalChunks: metadatas[i]?.totalChunks || 0,
                documentType: metadatas[i]?.documentType,
                title: metadatas[i]?.title,
                fileSize: metadatas[i]?.fileSize,
                fileName: metadatas[i]?.fileName
              }
            });
          }
        }
        console.log(`Included all results: ${searchResults.length} found`);
      }

      // Sort by score descending and limit
      searchResults.sort((a, b) => b.score - a.score);
      return searchResults.slice(0, limit);

    } catch (error) {
      console.error(`Search failed in collection: ${collectionName}`, error);
      throw new Error(`Search failed: ${(error as Error).message}`);
    }
  }

  /**
   * Delete a collection
   */
  async deleteCollection(name: string): Promise<void> {
    await this.ensureInitialized();

    try {
      if (this.collections.has(name)) {
        await this.client.deleteCollection({ name });
        this.collections.delete(name);
        console.log(`Deleted collection: ${name}`);
      }
    } catch (error) {
      console.error(`Failed to delete collection: ${name}`, error);
      throw new Error(`Delete collection failed: ${(error as Error).message}`);
    }
  }

  /**
   * List all collections
   */
  async listCollections(): Promise<string[]> {
    await this.ensureInitialized();

    try {
      await this.loadCollections();
      // Filter out invalid collection names
      return Array.from(this.collections.keys()).filter(name => 
        name && name !== 'undefined' && name.trim()
      );
    } catch (error) {
      console.error('Failed to list collections', error);
      return [];
    }
  }

  /**
   * Get documents in a collection
   */
  async getCollectionDocuments(collectionName: string): Promise<Array<{
    id: string;
    fileName: string;
    title?: string;
    chunkCount: number;
    fileSize?: number;
    documentType?: string;
  }>> {
    await this.ensureInitialized();

    try {
      if (!this.collections.has(collectionName)) {
        return [];
      }

      const collection = this.collections.get(collectionName);
      
      // Get all documents from collection
      const result = await collection.get({ limit: 10000 });
      
      if (!result.ids || result.ids.length === 0) {
        return [];
      }

      // Group chunks by document (using documentId from metadata or source)
      const documentsMap = new Map<string, {
        id: string;
        fileName: string;
        title?: string;
        chunkCount: number;
        fileSize?: number;
        documentType?: string;
      }>();

      const ids = result.ids;
      const metadatas = result.metadatas || [];

      for (let i = 0; i < ids.length; i++) {
        const metadata = metadatas[i] || {};
        const chunkId = ids[i] as string;
        
        // Extract document ID from chunk ID (format: documentId_chunk_index)
        const docIdMatch = chunkId.match(/^([^_]+)_chunk_/);
        const docId = docIdMatch ? docIdMatch[1] : chunkId;
        
        const fileName = metadata.fileName as string || metadata.source as string || 'Unknown';
        const title = metadata.title as string;
        const fileSize = metadata.fileSize as number;
        const documentType = metadata.documentType as string;

        if (!documentsMap.has(docId)) {
          documentsMap.set(docId, {
            id: docId,
            fileName,
            title,
            chunkCount: 0,
            fileSize,
            documentType
          });
        }

        const doc = documentsMap.get(docId)!;
        doc.chunkCount++;
      }

      return Array.from(documentsMap.values());
    } catch (error) {
      console.error(`Failed to get documents for collection: ${collectionName}`, error);
      return [];
    }
  }

  /**
   * Delete documents by document ID from a collection
   */
  async deleteDocuments(collectionName: string, documentIds: string[]): Promise<void> {
    await this.ensureInitialized();

    try {
      if (!this.collections.has(collectionName)) {
        throw new Error(`Collection ${collectionName} does not exist`);
      }

      const collection = this.collections.get(collectionName);
      
      // Get all chunks and filter by document IDs
      const result = await collection.get({ limit: 10000 });
      
      if (!result.ids || result.ids.length === 0) {
        return;
      }

      // Find all chunk IDs that belong to the documents to delete
      const idsToDelete: string[] = [];
      const ids = result.ids;

      for (const id of ids) {
        const idStr = id as string;
        // Check if this chunk belongs to any of the documents to delete
        for (const docId of documentIds) {
          if (idStr.startsWith(`${docId}_chunk_`)) {
            idsToDelete.push(idStr);
            break;
          }
        }
      }

      if (idsToDelete.length > 0) {
        await collection.delete({ ids: idsToDelete });
        console.log(`Deleted ${idsToDelete.length} chunks from ${documentIds.length} documents`);
      }
    } catch (error) {
      console.error(`Failed to delete documents from collection: ${collectionName}`, error);
      throw new Error(`Delete documents failed: ${(error as Error).message}`);
    }
  }

  /**
   * Get collection statistics
   */
  async getCollectionStats(name: string): Promise<{
    name: string;
    count: number;
  } | null> {
    await this.ensureInitialized();

    try {
      if (!this.collections.has(name)) {
        return null;
      }

      const collection = this.collections.get(name);
      
      // Try to get count - Chroma API might vary
      let count = 0;
      try {
        // Try count() method first
        if (typeof collection.count === 'function') {
          count = await collection.count();
        } else {
          // Fallback: get all IDs and count them
          const result = await collection.get({ limit: 10000 });
          count = result.ids?.length || 0;
        }
      } catch (countError) {
        // If count fails, try get() as fallback
        try {
          const result = await collection.get({ limit: 10000 });
          count = result.ids?.length || 0;
        } catch (getError) {
          console.warn(`Could not get count for collection ${name}`, getError);
          count = 0;
        }
      }

      return {
        name,
        count
      };
    } catch (error) {
      console.error(`Failed to get stats for collection: ${name}`, error);
      return null;
    }
  }
}

// Singleton instance
export const vectorDb = new ChromaVectorDB();

