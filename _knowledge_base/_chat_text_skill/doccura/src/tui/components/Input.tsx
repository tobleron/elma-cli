import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';
import TextInput from 'ink-text-input';

interface InputProps {
  onSubmit: (value: string) => void;
  isProcessing: boolean;
}

export function Input({ onSubmit, isProcessing }: InputProps) {
  const [value, setValue] = useState('');

  const handleSubmit = () => {
    if (value.trim() && !isProcessing) {
      onSubmit(value);
      setValue('');
    }
  };

  return (
    <Box borderTop={true} paddingX={1} paddingY={0}>
      <Text color="gray">{'> '}</Text>
      <TextInput
        value={value}
        onChange={setValue}
        onSubmit={handleSubmit}
        placeholder={isProcessing ? 'Processing...' : 'Type your question or /help for commands'}
      />
    </Box>
  );
}

