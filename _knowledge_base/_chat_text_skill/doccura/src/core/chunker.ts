import { config } from '../config';

export class TextChunker {
  /**
   * Split text into overlapping chunks
   */
  chunkText(text: string, chunkSize?: number, overlap?: number): string[] {
    const size = chunkSize || config.rag.chunkSize;
    const overlapSize = overlap || config.rag.chunkOverlap;

    if (size <= 0) {
      throw new Error('Chunk size must be positive');
    }

    if (overlapSize >= size) {
      throw new Error('Overlap size must be less than chunk size');
    }

    const chunks: string[] = [];
    const sentences = this.splitIntoSentences(text);

    let currentChunk = '';
    let currentSize = 0;

    for (const sentence of sentences) {
      const sentenceSize = sentence.length;

      // If adding this sentence would exceed chunk size
      if (currentSize + sentenceSize > size && currentChunk.trim()) {
        chunks.push(currentChunk.trim());
        // Start new chunk with overlap from previous chunk
        currentChunk = this.getOverlapText(currentChunk, overlapSize);
        currentSize = currentChunk.length;
      }

      currentChunk += sentence;
      currentSize += sentenceSize;
    }

    // Add remaining chunk
    if (currentChunk.trim()) {
      chunks.push(currentChunk.trim());
    }

    return chunks;
  }

  /**
   * Split text into sentences (basic implementation)
   */
  private splitIntoSentences(text: string): string[] {
    // Split on sentence endings, but keep the punctuation
    const sentences = text
      .split(/(?<=[.!?])\s+/)
      .filter(sentence => sentence.trim().length > 0)
      .map(sentence => sentence.trim() + ' ');

    return sentences;
  }

  /**
   * Get overlapping text from the end of a chunk
   */
  private getOverlapText(chunk: string, overlapSize: number): string {
    if (chunk.length <= overlapSize) {
      return chunk;
    }

    // Find a good breaking point near the overlap size
    let breakPoint = overlapSize;

    // Try to break at word boundary
    for (let i = overlapSize; i > overlapSize - 50 && i > 0; i--) {
      if (chunk[i] === ' ') {
        breakPoint = i + 1; // Include the space
        break;
      }
    }

    return chunk.slice(-breakPoint);
  }

  /**
   * Get chunk statistics
   */
  getChunkStats(chunks: string[]): {
    totalChunks: number;
    averageChunkSize: number;
    minChunkSize: number;
    maxChunkSize: number;
  } {
    if (chunks.length === 0) {
      return {
        totalChunks: 0,
        averageChunkSize: 0,
        minChunkSize: 0,
        maxChunkSize: 0
      };
    }

    const sizes = chunks.map(chunk => chunk.length);
    const totalSize = sizes.reduce((sum, size) => sum + size, 0);

    return {
      totalChunks: chunks.length,
      averageChunkSize: Math.round(totalSize / chunks.length),
      minChunkSize: Math.min(...sizes),
      maxChunkSize: Math.max(...sizes)
    };
  }
}

