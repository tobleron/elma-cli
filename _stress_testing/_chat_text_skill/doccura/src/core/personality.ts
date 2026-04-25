import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { cwd } from 'process';

const DEFAULT_PERSONALITY = 'You are a friendly and helpful AI assistant. Answer questions and help the user.';

/**
 * Load personality from personality.txt file
 * Falls back to default if file doesn't exist
 */
export function getPersonality(): string {
  const personalityPath = process.env.PERSONALITY_FILE || join(cwd(), 'personality.txt');
  
  try {
    if (existsSync(personalityPath)) {
      const content = readFileSync(personalityPath, 'utf-8').trim();
      if (content) {
        return content;
      }
    }
  } catch (error) {
    console.warn(`Warning: Could not load personality from ${personalityPath}, using default.`);
  }
  
  return DEFAULT_PERSONALITY;
}

/**
 * Get RAG-specific personality (can be different from chat personality)
 */
export function getRAGPersonality(): string {
  const ragPersonalityPath = process.env.RAG_PERSONALITY_FILE || join(cwd(), 'rag-personality.txt');
  
  try {
    if (existsSync(ragPersonalityPath)) {
      const content = readFileSync(ragPersonalityPath, 'utf-8').trim();
      if (content) {
        return content;
      }
    }
  } catch (error) {
    // Fall back to regular personality if RAG-specific doesn't exist
  }
  
  // Default RAG personality
  return `You are an AI assistant that answers questions based on the provided documents. 
Use only the information from the context to answer. If you cannot find the answer in the context, say that you don't have enough information.
Cite sources when possible (e.g., [Source, Page X]).`;
}

