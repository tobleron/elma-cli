#!/usr/bin/env bun

import 'dotenv/config';
import { render } from 'ink';
import React from 'react';
import { config } from './src/config';
import { ragService } from './src/core/rag-service';
import { ollamaClient } from './src/ollama/client';
import { App } from './src/tui/app';

async function checkDockerService(): Promise<boolean> {
  try {
    const { execSync } = await import('child_process');
    execSync('docker ps', { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

async function checkChromaContainer(): Promise<boolean> {
  try {
    const { execSync } = await import('child_process');
    const output = execSync('docker ps --format "{{.Names}}"', { encoding: 'utf-8' });
    return output.includes('doccura-chroma') || output.includes('chroma');
  } catch {
    return false;
  }
}

async function startOllamaIfNeeded(): Promise<boolean> {
  try {
    const { execSync } = await import('child_process');
    const { cwd } = await import('process');
    const { join } = await import('path');
    const { readFileSync, existsSync } = await import('fs');
    
    // Check if Ollama is already running
    try {
      execSync('curl -s http://localhost:11434/api/tags > /dev/null', { stdio: 'ignore' });
      return false; // Already running, we didn't start it
    } catch {
      // Not running, start it
    }
    
    // Check if PID file exists (Ollama was started by us before)
    const pidFile = join(cwd(), '.ollama.pid');
    const wasStartedByUs = existsSync(pidFile);
    
    // Start Ollama using the project script
    const scriptPath = join(cwd(), 'scripts', 'start-ollama.sh');
    execSync(`bash "${scriptPath}"`, { stdio: 'ignore' });
    
    // Wait a bit for Ollama to start
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // Verify it started
    let retries = 5;
    while (retries > 0) {
      try {
        execSync('curl -s http://localhost:11434/api/tags > /dev/null', { stdio: 'ignore' });
        return true; // Successfully started by us
      } catch {
        retries--;
        await new Promise(resolve => setTimeout(resolve, 1000));
      }
    }
    return false;
  } catch (error) {
    // If script fails, continue anyway - maybe Ollama is already running
    return false;
  }
}

async function stopOllama(): Promise<void> {
  try {
    const { execSync } = await import('child_process');
    const { cwd } = await import('process');
    const { join } = await import('path');
    
    const scriptPath = join(cwd(), 'scripts', 'stop-ollama.sh');
    execSync(`bash "${scriptPath}"`, { stdio: 'ignore' });
  } catch (error) {
    // Ignore errors when stopping
  }
}

async function main() {
  let ollamaStartedByUs = false;
  
  // Setup cleanup handlers
  let cleanupDone = false;
  const cleanup = async () => {
    if (cleanupDone) return;
    cleanupDone = true;
    
    if (ollamaStartedByUs) {
      console.log('\n\nStopping Ollama...');
      await stopOllama();
    }
  };
  
  process.once('SIGINT', async () => {
    await cleanup();
    process.exit(0);
  });
  
  process.once('SIGTERM', async () => {
    await cleanup();
    process.exit(0);
  });
  
  // Also handle beforeExit (when process is about to exit normally)
  process.once('beforeExit', async () => {
    await cleanup();
  });
  
  try {
    // Validate config
    config.validate();

    console.log('Initializing Ollama RAG TUI...\n');

    // Start Ollama if needed
    process.stdout.write('Checking Ollama server... ');
    const ollamaHealthy = await ollamaClient.checkHealth();
    if (!ollamaHealthy) {
      console.log('⚠ (not running, starting...)');
      process.stdout.write('Starting Ollama... ');
      ollamaStartedByUs = await startOllamaIfNeeded();
      
      // Check again after starting
      const retryHealthy = await ollamaClient.checkHealth();
      if (!retryHealthy) {
        console.error('❌');
        console.error(`\n❌ Failed to start Ollama server at ${config.ollama.endpoint}`);
        console.error('Please start it manually: ./scripts/start-ollama.sh');
        process.exit(1);
      }
      console.log('\x1b[32m✓\x1b[0m');
    } else {
      console.log('\x1b[32m✓\x1b[0m');
    }

    // Check Docker service
    process.stdout.write('Checking Docker service... ');
    const dockerRunning = await checkDockerService();
    if (!dockerRunning) {
      console.error('❌');
      console.error('\n❌ Docker service is not running');
      console.error('Please start Docker: sudo systemctl start docker');
      process.exit(1);
    }
    console.log('\x1b[32m✓\x1b[0m');

    // Check Chroma container
    process.stdout.write('Checking Chroma container... ');
    const chromaRunning = await checkChromaContainer();
    if (!chromaRunning) {
      console.log('⚠ (not found, will attempt to start)');
      process.stdout.write('Starting Chroma container... ');
      try {
        const { execSync } = await import('child_process');
        const { cwd } = await import('process');
        execSync('docker-compose up -d chroma', { 
          cwd: `${cwd()}/docker`,
          stdio: 'ignore'
        });
        // Wait a bit for container to start
        await new Promise(resolve => setTimeout(resolve, 2000));
        console.log('\x1b[32m✓\x1b[0m');
      } catch (error) {
        console.error('❌');
        console.error('\n❌ Failed to start Chroma container');
        console.error('Please start it manually: cd docker && docker-compose up -d chroma');
        process.exit(1);
      }
    } else {
      console.log('\x1b[32m✓\x1b[0m');
    }

    console.log(`\nModel: ${config.ollama.model}`);
    console.log(`Thinking enabled: ${config.ollama.enableThinking}\n`);

    // Initialize RAG service
    process.stdout.write('Initializing Chroma Vector DB... ');
    await ragService.initialize();
    console.log('\x1b[32m✓\x1b[0m\n');

    // Start TUI
    render(<App />);
  } catch (error) {
    console.error('❌');
    console.error('Failed to start application:', error);
    process.exit(1);
  }
}

main();
