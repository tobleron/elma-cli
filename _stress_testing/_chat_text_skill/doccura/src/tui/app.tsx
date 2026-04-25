import React, { useState, useEffect, useCallback } from 'react';
import { Box, Text } from 'ink';
import { ollamaClient, OllamaMessage } from '../ollama/client';
import { ragChat } from '../ollama/rag-chat';
import { ragService } from '../core/rag-service';
import { config } from '../config';
import { getPersonality } from '../core/personality';
import { ChatView } from './components/ChatView';
import { Input } from './components/Input';
import { StatusBar } from './components/StatusBar';

export interface Message {
  role: 'user' | 'assistant';
  content: string;
  sources?: any[];
}

export function App() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [currentCollection, setCurrentCollection] = useState<string>('default');
  const [isProcessing, setIsProcessing] = useState(false);
  const [ollamaStatus, setOllamaStatus] = useState<'online' | 'offline'>('offline');
  const [collections, setCollections] = useState<string[]>([]);

  // Check Ollama status on mount
  useEffect(() => {
    checkOllamaStatus();
    loadCollections();
    
    // Add welcome message
    setMessages([{
      role: 'assistant',
      content: `Welcome! I'm ready to answer questions about your documents.\nUse "/help" for available commands.`
    }]);
  }, []);

  // Sync currentCollection when collections change
  useEffect(() => {
    if (collections.length > 0 && (!currentCollection || !collections.includes(currentCollection))) {
      setCurrentCollection(collections[0]);
    }
  }, [collections]);

  const checkOllamaStatus = async () => {
    const healthy = await ollamaClient.checkHealth();
    setOllamaStatus(healthy ? 'online' : 'offline');
  };

  const loadCollections = async () => {
    try {
      const cols = await ragService.listCollections();
      const collectionNames = cols.map(c => c.name);
      setCollections(collectionNames);
      
      // Set current collection if not set or if current one doesn't exist
      if (collectionNames.length > 0) {
        if (!currentCollection || !collectionNames.includes(currentCollection)) {
          setCurrentCollection(collectionNames[0]);
        }
      } else {
        // No collections, reset to default
        setCurrentCollection('default');
      }
    } catch (error) {
      console.error('Failed to load collections', error);
    }
  };

  const handleCommand = async (input: string) => {
    const trimmed = input.trim();
    if (!trimmed) return;

    // Check for RAG prefix first (before commands)
    const wantsRAG = trimmed.startsWith('@rag ') || trimmed.startsWith('/rag ') || trimmed.startsWith('?');
    if (wantsRAG && !trimmed.startsWith('/rag ')) {
      // Handle @rag and ? - these are not commands
      // Will be handled in query section
    }

    // Handle commands (but skip /rag as it's a RAG prefix, not a command)
    if (trimmed.startsWith('/') && !trimmed.startsWith('/rag ')) {
      // Parse command with proper quote handling
      const parseArgs = (str: string): string[] => {
        const args: string[] = [];
        let current = '';
        let inQuotes = false;
        let quoteChar = '';

        for (let i = 0; i < str.length; i++) {
          const char = str[i];
          
          if ((char === '"' || char === "'") && !inQuotes) {
            inQuotes = true;
            quoteChar = char;
          } else if (char === quoteChar && inQuotes) {
            inQuotes = false;
            quoteChar = '';
          } else if (char === ' ' && !inQuotes) {
            if (current.trim()) {
              args.push(current.trim());
              current = '';
            }
          } else {
            current += char;
          }
        }
        
        if (current.trim()) {
          args.push(current.trim());
        }
        
        return args;
      };

      const commandPart = trimmed.slice(1).trim();
      const spaceIndex = commandPart.indexOf(' ');
      const cmd = spaceIndex === -1 ? commandPart : commandPart.slice(0, spaceIndex);
      const argsStr = spaceIndex === -1 ? '' : commandPart.slice(spaceIndex + 1);
      const args = argsStr ? parseArgs(argsStr) : [];

      switch (cmd) {
        case 'help':
          setMessages(prev => [...prev, {
            role: 'assistant',
            content: `Available commands:\n` +
              `\x1b[32m/help\x1b[0m - Show this message\n` +
              `\x1b[32m/upload <filepath> [collection]\x1b[0m - Upload PDF/TXT\n` +
              `\x1b[32m/collections\x1b[0m - List collections with documents\n` +
              `\x1b[32m/collection <name>\x1b[0m - Switch active collection\n` +
              `\x1b[32m/status\x1b[0m - Show system status\n` +
              `\x1b[32m/coldel <name>\x1b[0m - Delete collection\n` +
              `\x1b[32m/col <name> del <num>\x1b[0m - Delete document from collection\n` +
              `\x1b[32m/exit\x1b[0m or \x1b[32m/bye\x1b[0m - Exit the application\n\n` +
              `For RAG queries (search in documents):\n` +
              `\x1b[32m@rag <question>\x1b[0m - Search in documents\n` +
              `\x1b[32m/rag <question>\x1b[0m - Search in documents\n` +
              `\x1b[32m? <question>\x1b[0m - Search in documents\n\n` +
              `For normal conversation, type directly without prefix.`
          }]);
          return;

        case 'exit':
        case 'bye':
          setMessages(prev => [...prev, {
            role: 'assistant',
            content: 'Goodbye! 👋'
          }]);
          // Give a moment for the message to display, then exit
          setTimeout(() => {
            process.exit(0);
          }, 500);
          return;

        case 'upload':
          if (args.length === 0) {
            setMessages(prev => [...prev, {
              role: 'assistant',
              content: 'Usage: /upload <filepath> [collection]\nExample: /upload /path/to/document.pdf\nExample: /upload /path/to/document.pdf mycollection\n\nNote: File must have .pdf or .txt extension'
            }]);
          } else {
            setIsProcessing(true);
            let filePath = args[0];
            const collection = args[1] || currentCollection;
            
            try {
              // Try to find file if extension is missing
              const fs = await import('fs');
              const path = await import('path');
              
              if (!fs.existsSync(filePath)) {
                // Try with .pdf extension
                const withPdf = filePath + '.pdf';
                if (fs.existsSync(withPdf)) {
                  filePath = withPdf;
                } else {
                  // Try with .txt extension
                  const withTxt = filePath + '.txt';
                  if (fs.existsSync(withTxt)) {
                    filePath = withTxt;
                  }
                }
              }

              // Clean up file path (remove quotes if present, trim)
              filePath = filePath.replace(/^["']|["']$/g, '').trim();

              // Validate file exists
              if (!fs.existsSync(filePath)) {
                // Try to find similar files
                const dir = path.dirname(filePath);
                const baseName = path.basename(filePath);
                
                let suggestion = '';
                try {
                  if (fs.existsSync(dir)) {
                    const files = fs.readdirSync(dir);
                    const matches = files.filter(f => 
                      f.toLowerCase().includes(baseName.toLowerCase()) || 
                      baseName.toLowerCase().includes(f.toLowerCase().split('.')[0])
                    );
                    if (matches.length > 0) {
                      suggestion = `\n\nSimilar files found:\n${matches.slice(0, 5).map(f => `  - ${path.join(dir, f)}`).join('\n')}`;
                    }
                  }
                } catch (e) {
                  // Ignore
                }
                
                throw new Error(`File does not exist: ${filePath}${suggestion}\n\nMake sure:\n- Path is complete and correct\n- File has .pdf or .txt extension\n- You have read permissions`);
              }

              // Check extension
              const ext = path.extname(filePath).toLowerCase();
              if (ext !== '.pdf' && ext !== '.txt') {
                throw new Error(`File must have .pdf or .txt extension. You specified: ${ext || 'no extension'}`);
              }

              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `Uploading document: ${filePath}...`
              }]);

              const result = await ragService.indexDocument(
                filePath,
                collection
              );

              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `✅ Document uploaded successfully!\nDocument ID: ${result.documentId}\nChunks: ${result.chunksCount}\nProcessing time: ${result.processingTime}ms\nCollection: ${collection}`
              }]);

              // Reload collections to update UI
              await loadCollections();

            } catch (error) {
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `❌ Upload error: ${(error as Error).message}`
              }]);
            } finally {
              setIsProcessing(false);
            }
          }
          return;

        case 'collections':
          try {
            await loadCollections(); // Refresh first
            const cols = await ragService.listCollections(true); // Include documents
            if (cols.length === 0) {
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: 'No collections exist. Use /upload to upload documents.'
              }]);
            } else {
              let tree = '';
              for (const col of cols) {
                tree += `📁 ${col.name} (${col.chunkCount} chunks, ${col.documentCount} documents)\n`;
                if (col.documents && col.documents.length > 0) {
                  col.documents.forEach((doc, idx) => {
                    const fileName = doc.title || doc.fileName || 'Unknown';
                    tree += `   ${idx + 1}. ${fileName} (${doc.chunkCount} chunks)\n`;
                  });
                }
              }
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `Available collections (${cols.length}):\n${tree.trim()}`
              }]);
            }
          } catch (error) {
            setMessages(prev => [...prev, {
              role: 'assistant',
              content: `Error: ${(error as Error).message}`
            }]);
          }
          return;

        case 'col':
        case 'collection':
          if (args.length === 0) {
            setMessages(prev => [...prev, {
              role: 'assistant',
              content: `Active collection: ${currentCollection}`
            }]);
          } else if (args.length >= 2 && args[1] === 'del') {
            // Delete document from collection: /col <name> del <num>
            const collectionName = args[0];
            const docNum = parseInt(args[2]);
            
            if (isNaN(docNum) || docNum < 1) {
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: 'Invalid number. Use: /col <name> del <num> (num = number from /collections)'
              }]);
              return;
            }

            try {
              setIsProcessing(true);
              const documents = await ragService.getCollectionDocuments(collectionName);
              
              if (docNum > documents.length) {
                setMessages(prev => [...prev, {
                  role: 'assistant',
                  content: `Document #${docNum} does not exist. Collection has ${documents.length} documents.`
                }]);
                setIsProcessing(false);
                return;
              }

              const docToDelete = documents[docNum - 1];
              await ragService.deleteDocuments(collectionName, [docToDelete.id]);
              
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `✅ Document "${docToDelete.fileName}" deleted from collection "${collectionName}"`
              }]);
              
              await loadCollections();
            } catch (error) {
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `❌ Error: ${(error as Error).message}`
              }]);
            } finally {
              setIsProcessing(false);
            }
          } else {
            const newCollection = args[0];
            if (collections.includes(newCollection)) {
              setCurrentCollection(newCollection);
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `Collection changed to: ${newCollection}`
              }]);
            } else {
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `Collection "${newCollection}" does not exist. Use /collections to see available collections.`
              }]);
            }
          }
          return;

        case 'coldel':
          if (args.length === 0) {
            setMessages(prev => [...prev, {
              role: 'assistant',
              content: 'Usage: /coldel <collection_name>\nExample: /coldel test1'
            }]);
          } else {
            const collectionName = args[0];
            setIsProcessing(true);
            try {
              await ragService.deleteCollection(collectionName);
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `✅ Collection "${collectionName}" has been deleted`
              }]);
              await loadCollections();
              if (currentCollection === collectionName) {
                const cols = await ragService.listCollections();
                setCurrentCollection(cols.length > 0 ? cols[0].name : 'default');
              }
            } catch (error) {
              setMessages(prev => [...prev, {
                role: 'assistant',
                content: `❌ Error: ${(error as Error).message}`
              }]);
            } finally {
              setIsProcessing(false);
            }
          }
          return;

        case 'status':
          await checkOllamaStatus();
          await loadCollections(); // Refresh collections
          const cols = await ragService.listCollections();
          const currentCol = cols.find(c => c.name === currentCollection);
          
          setMessages(prev => [...prev, {
            role: 'assistant',
            content: `Status:\n` +
              `${ollamaStatus === 'online' ? '🟢' : '🔴'} Ollama: \x1b[32m${ollamaStatus}\x1b[0m\n` +
              `📦 Model: \x1b[32m${config.ollama.model}\x1b[0m\n` +
              `📚 Active collection: \x1b[32m${currentCollection || 'none'}\x1b[0m\n` +
              `📊 Total collections: \x1b[32m${cols.length}\x1b[0m\n` +
              `🔢 Chunks in collection: \x1b[32m${currentCol?.chunkCount || 0}\x1b[0m`
          }]);
          return;

        default:
          setMessages(prev => [...prev, {
            role: 'assistant',
            content: `Unknown command: ${cmd}. Use /help for help.`
          }]);
          return;
      }
    }

    // Handle query
    setIsProcessing(true);
    setMessages(prev => [...prev, { role: 'user', content: trimmed }]);

    try {
      // Check if user wants RAG query (prefix: @rag, /rag, or ?)
      const wantsRAG = trimmed.startsWith('@rag ') || trimmed.startsWith('/rag ') || trimmed.startsWith('?');
      const queryText = wantsRAG 
        ? trimmed.replace(/^(@rag |\/rag |\? )/, '').trim()
        : trimmed;

      // If RAG is requested, check for documents
      if (wantsRAG) {
        // Ensure we have a valid collection
        await loadCollections();
        const cols = await ragService.listCollections();
        
        if (cols.length === 0) {
          setMessages(prev => [...prev, {
            role: 'assistant',
            content: 'No collections available. Upload a document first with /upload.'
          }]);
          setIsProcessing(false);
          return;
        }

        // Set collection if not set
        if (!currentCollection || currentCollection === 'undefined' || !cols.find(c => c.name === currentCollection)) {
          setCurrentCollection(cols[0].name);
        }

        const currentCol = cols.find(c => c.name === currentCollection);
        const hasDocuments = currentCol && currentCol.chunkCount > 0;
        
        console.log(`RAG Query in collection: ${currentCollection}, hasDocuments: ${hasDocuments}, chunkCount: ${currentCol?.chunkCount || 0}`);

        if (!hasDocuments) {
          setMessages(prev => [...prev, {
            role: 'assistant',
            content: `Collection "${currentCollection}" has no documents. Upload a document with /upload or use /collections to see available collections.`
          }]);
          setIsProcessing(false);
          return;
        }

        // Use RAG
        let fullAnswer = '';
        try {
          const queryPromise = (async () => {
            for await (const chunk of ragChat.queryStream({
              question: queryText,
              collection: currentCollection
            })) {
              fullAnswer += chunk;
              // Update last message with streaming content
              setMessages(prev => {
                const newMessages = [...prev];
                const lastMsg = newMessages[newMessages.length - 1];
                if (lastMsg && lastMsg.role === 'assistant') {
                  lastMsg.content = fullAnswer;
                } else {
                  newMessages.push({ role: 'assistant', content: fullAnswer });
                }
                return newMessages;
              });
            }
          })();

          // Add timeout (5 minutes)
          await Promise.race([
            queryPromise,
            new Promise((_, reject) => 
              setTimeout(() => reject(new Error('Query timeout - attempt too long')), 5 * 60 * 1000)
            )
          ]);

          // Final update
          setMessages(prev => {
            const newMessages = [...prev];
            const lastMsg = newMessages[newMessages.length - 1];
            if (lastMsg && lastMsg.role === 'assistant') {
              lastMsg.content = fullAnswer;
            }
            return newMessages;
          });
        } catch (queryError) {
          throw new Error(`RAG query error: ${(queryError as Error).message}`);
        }
      } else {
        // Normal chat - no RAG prefix
        let fullAnswer = '';
        // Normal chat - direct with Ollama
        const messages: OllamaMessage[] = [
          { 
            role: 'system', 
            content: getPersonality()
          },
          { role: 'user', content: queryText }
        ];

        for await (const chunk of ollamaClient.chatStream(messages, { temperature: 0.7 })) {
          fullAnswer += chunk;
          // Update last message with streaming content
          setMessages(prev => {
            const newMessages = [...prev];
            const lastMsg = newMessages[newMessages.length - 1];
            if (lastMsg && lastMsg.role === 'assistant') {
              lastMsg.content = fullAnswer;
            } else {
              newMessages.push({ role: 'assistant', content: fullAnswer });
            }
            return newMessages;
          });
        }

        // Final update
        setMessages(prev => {
          const newMessages = [...prev];
          const lastMsg = newMessages[newMessages.length - 1];
          if (lastMsg && lastMsg.role === 'assistant') {
            lastMsg.content = fullAnswer;
          }
          return newMessages;
        });
      }

    } catch (error) {
      setMessages(prev => [...prev, {
        role: 'assistant',
        content: `Error: ${(error as Error).message}`
      }]);
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <Box flexDirection="column" height="100%">
      <Box flexGrow={1} flexDirection="column">
        <ChatView messages={messages} />
      </Box>
      <StatusBar
        collection={currentCollection}
        ollamaStatus={ollamaStatus}
        model={config.ollama.model}
        isProcessing={isProcessing}
      />
      <Input onSubmit={handleCommand} isProcessing={isProcessing} />
    </Box>
  );
}

