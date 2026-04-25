import { ollamaClient, OllamaMessage } from './client';
import { vectorDb } from '../core/vector-db';
import { embeddingService } from '../core/embeddings';
import { config } from '../config';
import { getRAGPersonality } from '../core/personality';
import { QueryRequest, QueryResponse, SearchResult } from '../types';

export class RAGChat {
  /**
   * Query RAG system and get answer from Ollama
   */
  async query(request: QueryRequest): Promise<QueryResponse> {
    const startTime = Date.now();

    try {
      const { question, collection, limit, threshold } = request;

      console.log(`Processing RAG query: "${question}" in collection: ${collection}`);

      // Generate query embedding
      const queryEmbedding = await embeddingService.generateQueryEmbedding(question);

      // Search vector database with lower threshold for better results
      const searchThreshold = threshold || Math.min(config.rag.similarityThreshold, 0.2);
      let searchResults = await vectorDb.search(
        collection,
        queryEmbedding,
        (limit || config.rag.maxResults) * 2,
        searchThreshold
      );

      if (searchResults.length === 0) {
        // Try with even lower threshold
        const fallbackResults = await vectorDb.search(
          collection,
          queryEmbedding,
          limit || config.rag.maxResults,
          0.1 // Very low threshold as fallback
        );
        
        if (fallbackResults.length === 0) {
          return {
            answer: 'I could not find relevant information in the available documents. Try rephrasing your question or be more specific.',
            sources: [],
            processingTime: Date.now() - startTime
          };
        }
        
        searchResults = fallbackResults.slice(0, limit || config.rag.maxResults);
      } else {
        // Limit to requested number
        searchResults = searchResults.slice(0, limit || config.rag.maxResults);
      }

      // Format context for LLM
      const context = this.formatContext(searchResults);

      // Create prompt with context (using personality file)
      const systemPrompt = getRAGPersonality();

      const userPrompt = `Context from documents:
${context}

Question: ${question}

Answer the question using only the information from the context above.`;

      const messages: OllamaMessage[] = [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: userPrompt }
      ];

      // Get answer from Ollama
      const answer = await ollamaClient.chat(messages, { temperature: 0.7 });

      const processingTime = Date.now() - startTime;

      console.log(`RAG query completed in ${processingTime}ms (${searchResults.length} sources)`);

      return {
        answer,
        sources: searchResults,
        processingTime
      };

    } catch (error) {
      console.error('RAG query failed', error);
      throw new Error(`RAG query failed: ${(error as Error).message}`);
    }
  }

  /**
   * Query RAG system and stream answer from Ollama
   */
  async *queryStream(request: QueryRequest): AsyncGenerator<string, void, unknown> {
    try {
      const { question, collection, limit, threshold } = request;

      console.log(`Processing RAG query (streaming): "${question}" in collection: ${collection}`);

      // Generate query embedding
      const queryEmbedding = await embeddingService.generateQueryEmbedding(question);

      // Search vector database with very low threshold to get results
      // Start with very low threshold (0.1) to ensure we get results
      const searchThreshold = threshold || 0.1;
      let searchResults = await vectorDb.search(
        collection,
        queryEmbedding,
        (limit || config.rag.maxResults) * 5, // Get many results initially
        searchThreshold
      );

      console.log(`RAG query: "${question}", found ${searchResults.length} results with threshold ${searchThreshold}`);

      if (searchResults.length === 0) {
        // Try with even lower threshold (almost no filtering)
        console.log('Trying with minimal threshold (0.05)...');
        const fallbackResults = await vectorDb.search(
          collection,
          queryEmbedding,
          (limit || config.rag.maxResults) * 10,
          0.05 // Very minimal threshold
        );
        
        console.log(`Fallback search found ${fallbackResults.length} results`);
        
        if (fallbackResults.length === 0) {
          // Last resort: get any results, no threshold
          console.log('Last resort: getting top results without threshold filter...');
          const lastResort = await vectorDb.search(
            collection,
            queryEmbedding,
            limit || config.rag.maxResults,
            0 // No threshold at all
          );
          
          if (lastResort.length === 0) {
            yield 'I could not find relevant information in the available documents. Try rephrasing your question or be more specific.';
            return;
          }
          
          searchResults = lastResort;
        } else {
          searchResults = fallbackResults.slice(0, limit || config.rag.maxResults);
        }
      }

      // Format context for LLM
      const context = this.formatContext(searchResults);

      // Create prompt with context (using personality file)
      const systemPrompt = getRAGPersonality();

      const userPrompt = `Context from documents:
${context}

Question: ${question}

Answer the question using only the information from the context above.`;

      const messages: OllamaMessage[] = [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: userPrompt }
      ];

      // Stream answer from Ollama
      for await (const chunk of ollamaClient.chatStream(messages, { temperature: 0.7 })) {
        yield chunk;
      }

    } catch (error) {
      console.error('RAG query stream failed', error);
      throw new Error(`RAG query stream failed: ${(error as Error).message}`);
    }
  }

  /**
   * Format search results as context for LLM
   */
  private formatContext(results: SearchResult[]): string {
    return results
      .map((result, index) => {
        const source = result.metadata.source || 'Unknown document';
        const page = result.metadata.page || 0;
        return `[Source ${index + 1}: ${source}, Page ${page}]\n${result.content}`;
      })
      .join('\n\n---\n\n');
  }
}

// Singleton instance
export const ragChat = new RAGChat();

