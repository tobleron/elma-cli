import React from 'react';
import { Box, Text } from 'ink';

interface StatusBarProps {
  collection: string;
  ollamaStatus: 'online' | 'offline';
  model: string;
  isProcessing: boolean;
}

export function StatusBar({ collection, ollamaStatus, model, isProcessing }: StatusBarProps) {
  return (
    <Box borderTop={true} paddingX={1} paddingY={0}>
      <Text>
        <Text color="gray">Status: </Text>
        <Text color={ollamaStatus === 'online' ? 'green' : 'red'}>
          {ollamaStatus === 'online' ? '●' : '○'} Ollama
        </Text>
        <Text color="gray"> | </Text>
        <Text color="blue">Model: {model}</Text>
        <Text color="gray"> | </Text>
        <Text color="yellow">Collection: {collection}</Text>
        {isProcessing && (
          <>
            <Text color="gray"> | </Text>
            <Text color="magenta">Processing...</Text>
          </>
        )}
      </Text>
    </Box>
  );
}

