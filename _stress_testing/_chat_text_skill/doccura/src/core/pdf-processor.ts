import * as fs from 'fs';
import * as path from 'path';
import { config } from '../config';

// Use legacy build for Node.js compatibility
import { getDocument, GlobalWorkerOptions } from 'pdfjs-dist/legacy/build/pdf.mjs';

// Set worker source for PDF.js (legacy build for Node.js)
const pdfjsPath = path.dirname(require.resolve('pdfjs-dist/package.json'));
GlobalWorkerOptions.workerSrc = path.join(pdfjsPath, 'legacy/build/pdf.worker.mjs');

export class PDFProcessor {
  /**
   * Extract text from a PDF or TXT file
   */
  async extractText(filePath: string, originalFilename?: string): Promise<string> {
    const startTime = Date.now();

    try {
      console.log(`Starting document extraction: ${filePath}`);

      // Validate file exists
      if (!fs.existsSync(filePath)) {
        throw new Error(`File not found: ${filePath}`);
      }

      // Validate file size
      const stats = fs.statSync(filePath);
      const maxSizeBytes = config.rag.maxFileSizeMb * 1024 * 1024;
      if (stats.size > maxSizeBytes) {
        throw new Error(`File too large: ${stats.size} bytes (max: ${maxSizeBytes})`);
      }

      // Check if it's a text file
      const isTextFile = originalFilename ? originalFilename.toLowerCase().endsWith('.txt') : filePath.toLowerCase().endsWith('.txt');
      if (isTextFile) {
        console.log('Processing as text file');
        const text = fs.readFileSync(filePath, 'utf-8');
        const processingTime = Date.now() - startTime;
        console.log(`Text extraction completed in ${processingTime}ms`);
        return text;
      }

      // Load PDF document
      const data = new Uint8Array(fs.readFileSync(filePath));
      let pdf;
      try {
        pdf = await getDocument({ data }).promise;
      } catch (pdfError) {
        console.warn('Failed to load PDF document, it may be corrupted or invalid:', pdfError);
        return 'This PDF document could not be loaded. It may be corrupted, password-protected, or not a valid PDF file.';
      }

      console.log(`PDF loaded: ${pdf.numPages} pages`);

      let fullText = '';

      // Extract text from each page
      for (let pageNum = 1; pageNum <= pdf.numPages; pageNum++) {
        try {
          const page = await pdf.getPage(pageNum);
          const textContent = await page.getTextContent();

          const pageText = textContent.items
            .map((item: any) => item.str)
            .join(' ')
            .replace(/\s+/g, ' ') // Normalize whitespace
            .trim();

          if (pageText) {
            fullText += pageText + '\n\n';
          }

          // Log progress for large PDFs
          if (pageNum % 10 === 0) {
            console.log(`Processed ${pageNum}/${pdf.numPages} pages`);
          }
        } catch (pageError) {
          console.warn(`Failed to extract text from page ${pageNum}:`, pageError);
          // Continue with other pages
        }
      }

      if (!fullText.trim()) {
        console.warn('No text content found in PDF - it may be image-based or corrupted');
        return `This PDF document appears to be image-based or corrupted. It contains ${pdf.numPages} pages but no extractable text content.`;
      }

      const processingTime = Date.now() - startTime;
      console.log(`PDF extraction completed in ${processingTime}ms (${pdf.numPages} pages, ${fullText.length} chars)`);

      return fullText.trim();

    } catch (error) {
      console.error('PDF extraction failed', error);
      throw new Error(`Failed to extract text from PDF: ${(error as Error).message}`);
    }
  }

  /**
   * Get PDF metadata
   */
  async getMetadata(filePath: string): Promise<any> {
    try {
      const data = new Uint8Array(fs.readFileSync(filePath));
      const pdf = await getDocument({ data }).promise;

      const metadata = await pdf.getMetadata();
      const stats = fs.statSync(filePath);

      const info = metadata.info as any;
      return {
        pages: pdf.numPages,
        title: info?.Title || path.basename(filePath, '.pdf'),
        author: info?.Author,
        subject: info?.Subject,
        creator: info?.Creator,
        producer: info?.Producer,
        creationDate: info?.CreationDate,
        modificationDate: info?.ModDate,
        fileSize: stats.size,
        fileName: path.basename(filePath)
      };
    } catch (error) {
      console.warn(`Could not get PDF metadata: ${filePath}`, error);
      return null;
    }
  }

  /**
   * Validate document file (PDF or TXT)
   */
  validateFile(filePath: string, originalFilename?: string): boolean {
    try {
      // Check file extension (PDF or TXT)
      const filenameToCheck = originalFilename || filePath;
      const isPdf = filenameToCheck.toLowerCase().endsWith('.pdf');
      const isTxt = filenameToCheck.toLowerCase().endsWith('.txt');
      const isValidType = isPdf || isTxt;
      
      if (!isValidType) {
        return false;
      }

      // Check file exists
      if (!fs.existsSync(filePath)) {
        return false;
      }

      // Check file size
      const stats = fs.statSync(filePath);
      const maxSizeBytes = config.rag.maxFileSizeMb * 1024 * 1024;
      if (stats.size > maxSizeBytes) {
        return false;
      }

      // For PDF files, validate magic bytes for security
      if (isPdf) {
        const buffer = fs.readFileSync(filePath);
        // PDF files start with %PDF-
        if (buffer.toString('ascii', 0, 4) !== '%PDF') {
          return false;
        }
      }

      return true;
    } catch (error) {
      console.error('Document validation failed', error);
      return false;
    }
  }
}

