import React from 'react';
import { Box, Text, useStdout } from 'ink';
import { Message } from '../app';

interface ChatViewProps {
  messages: Message[];
}

function parseContent(content: string): React.ReactNode[] {
  // Check for thinking tags - try both <think> and <think>
  const thinkingRegex = /<(?:think|redacted_reasoning)>([\s\S]*?)<\/(?:think|redacted_reasoning)>/g;
  const parts: React.ReactNode[] = [];
  let lastIndex = 0;
  let match;
  let hasThinking = false;
  
  // Reset regex
  thinkingRegex.lastIndex = 0;
  
  while ((match = thinkingRegex.exec(content)) !== null) {
    hasThinking = true;
    
    // Add text before thinking (if any) - this is the actual answer (green)
    if (match.index > lastIndex) {
      const beforeText = content.substring(lastIndex, match.index);
      if (beforeText.trim()) {
        parts.push(
          <Text key={`text-${lastIndex}`} color="green">
            {beforeText}
          </Text>
        );
      }
    }

    // Add thinking (white) - trim trailing newlines to reduce spacing
    const thinkingText = match[1].replace(/\n+$/, '');
    parts.push(
      <Text key={`think-${match.index}`} color="white">
        {thinkingText}
      </Text>
    );

    lastIndex = match.index + match[0].length;
  }

  // Add remaining text after last thinking (if any) - this is the actual answer (green)
  // Add a small separator if there was thinking before
  if (lastIndex < content.length) {
    const afterText = content.substring(lastIndex).trim();
    if (afterText) {
      if (hasThinking) {
        // Add a single line break before answer if there was thinking
        parts.push(
          <Text key="break-after-thinking">{"\n"}</Text>
        );
      }
      parts.push(
        <Text key={`text-${lastIndex}`} color="green">
          {afterText}
        </Text>
      );
    }
  }

  // If no thinking found, return entire content with green color
  if (parts.length === 0) {
    return [
      <Text key="content" color="green">
        {content}
      </Text>
    ];
  }

  return parts;
}

export function ChatView({ messages }: ChatViewProps) {
  const { stdout } = useStdout();
  const terminalWidth = stdout?.columns || 80;
  
  return (
    <Box flexDirection="column" paddingX={1} paddingY={1}>
      {messages.map((msg, index) => {
        const hasAnsiCodes = msg.content?.includes('\x1b[') || false;
        const hasThinking = msg.content?.includes('<think>') || msg.content?.includes('<think>') || false;
        const borderColor = msg.role === 'user' ? 'cyan' : 'green';
        
        const title = `${msg.role === 'user' ? 'You' : 'Assistant'}:`;
        // Calculate border line width (account for padding and margins)
        const borderWidth = terminalWidth - 4; // Account for padding
        const titleLength = title.length;
        const borderAfterTitle = borderWidth - titleLength - 4; // 4 for "╭─ " and " ─"
        
        return (
          <Box key={index} flexDirection="column" marginBottom={1}>
            {/* Top border with title */}
            <Box flexDirection="row" marginLeft={1}>
              <Text color={borderColor}>╭─ </Text>
              <Text color={borderColor} bold>{title}</Text>
              <Text color={borderColor}>{'─'.repeat(Math.max(1, borderAfterTitle))}</Text>
              <Text color={borderColor}>╮</Text>
            </Box>
            
            {/* Box with left/right/bottom borders only (filled) */}
            <Box 
              borderLeft={true}
              borderRight={true}
              borderBottom={true}
              borderColor={borderColor}
              flexDirection="column"
              paddingX={1}
              paddingY={0}
              marginLeft={1}
              marginRight={1}
            >
              {/* Content */}
              <Box paddingY={0} flexDirection="column" marginTop={0}>
                {msg.role === 'assistant' ? (
                  hasAnsiCodes && !hasThinking ? (
                    // Content with ANSI codes (like /status) - display directly
                    <Text>{msg.content}</Text>
                  ) : (
                    // Content with thinking or normal - parse it
                    parseContent(msg.content || '')
                  )
                ) : (
                  <Text>{msg.content}</Text>
                )}
              </Box>
            </Box>
            
            {/* Bottom border */}
            <Box flexDirection="row" marginLeft={1}>
              <Text color={borderColor}>╰</Text>
              <Text color={borderColor}>{'─'.repeat(borderWidth - 2)}</Text>
              <Text color={borderColor}>╯</Text>
            </Box>
          </Box>
        );
      })}
    </Box>
  );
}

