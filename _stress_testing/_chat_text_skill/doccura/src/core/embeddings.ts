import { config } from '../config';

// Simple in-memory cache for embeddings
const embeddingCache = new Map<string, number[]>();

export class LocalEmbeddingService {
  private extractor: any = null;
  private initialized = false;

  /**
   * Initialize the embedding model
   */
  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    try {
      // Load the model - this might take a while on first run
      const { pipeline } = await import('@xenova/transformers');
      this.extractor = await pipeline('feature-extraction', config.rag.embeddingModel);

      this.initialized = true;

    } catch (error) {
      console.error('Failed to initialize embedding model', error);
      throw new Error(`Embedding model initialization failed: ${(error as Error).message}`);
    }
  }

  /**
   * Generate embeddings for multiple texts (with batching for performance)
   */
  async generateEmbeddings(texts: string[]): Promise<number[][]> {
    await this.ensureInitialized();

    const startTime = Date.now();
    const embeddings: number[][] = [];
    const BATCH_SIZE = 50; // Process 50 embeddings at a time

    try {
      console.log(`Generating embeddings for ${texts.length} texts (batched)`);

      for (let batchStart = 0; batchStart < texts.length; batchStart += BATCH_SIZE) {
        const batchEnd = Math.min(batchStart + BATCH_SIZE, texts.length);
        const batch = texts.slice(batchStart, batchEnd);
        
        // Process batch in parallel
        const batchEmbeddings = await Promise.all(
          batch.map(async (text) => {
            // Check cache first
            let embedding = embeddingCache.get(text);

            if (!embedding) {
              // Generate new embedding
              const output = await this.extractor(text, {
                pooling: 'mean',
                normalize: true
              });

              embedding = Array.from(output.data);

              // Cache the embedding
              embeddingCache.set(text, embedding);
            }

            return embedding;
          })
        );

        embeddings.push(...batchEmbeddings);

        // Log progress
        if (batchEnd % 100 === 0) {
          console.log(`Processed ${batchEnd}/${texts.length} embeddings`);
        }
      }

      const processingTime = Date.now() - startTime;
      console.log(`Embedding generation completed in ${processingTime}ms (${texts.length} texts)`);

      return embeddings;

    } catch (error) {
      console.error('Embedding generation failed', error);
      throw new Error(`Failed to generate embeddings: ${(error as Error).message}`);
    }
  }

  /**
   * Generate embedding for a single query
   */
  async generateQueryEmbedding(query: string): Promise<number[]> {
    await this.ensureInitialized();

    try {
      // Check cache first
      let embedding = embeddingCache.get(query);

      if (!embedding) {
        const output = await this.extractor(query, {
          pooling: 'mean',
          normalize: true
        });

        embedding = Array.from(output.data);

        // Cache the embedding
        embeddingCache.set(query, embedding);
      }

      return embedding;

    } catch (error) {
      console.error('Query embedding generation failed', error);
      throw new Error(`Failed to generate query embedding: ${(error as Error).message}`);
    }
  }

  /**
   * Ensure the model is initialized
   */
  private async ensureInitialized(): Promise<void> {
    if (!this.initialized) {
      await this.initialize();
    }
  }

  /**
   * Get model information
   */
  getModelInfo(): { name: string; initialized: boolean } {
    return {
      name: config.rag.embeddingModel,
      initialized: this.initialized
    };
  }

  /**
   * Clear cache
   */
  clearCache(): void {
    embeddingCache.clear();
    console.log('Embedding cache cleared');
  }
}

// Singleton instance
export const embeddingService = new LocalEmbeddingService();

