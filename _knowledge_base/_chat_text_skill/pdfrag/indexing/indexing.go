package indexing

import (
	"context"
	"crypto/md5"
	"database/sql"
	"fmt"
	"io"
	"math"
	"os"
	"path/filepath"
	"strings"
	"time"

	"pdfrag/chunking"
	"pdfrag/convert"
	"pdfrag/embeddings"
	"pdfrag/logging"

	"go.uber.org/zap"
)

// FileSignature captures file mod time and hash for change detection.
type FileSignature struct {
	ModTime time.Time
	Hash    string
}

// MetadataOverrides provides optional metadata overrides when indexing.
type MetadataOverrides struct {
	Title           *string
	Authors         *string
	PublicationDate *string
	DOI             *string
}

// IndexOptions controls indexing behavior.
type IndexOptions struct {
	MarkdownOptions    convert.MarkdownOptions
	MetadataOverrides  MetadataOverrides
	EmbeddingBatchSize int
	EmbedLimiter       chan struct{}
}

// IndexResult summarizes indexing output.
type IndexResult struct {
	DocumentID int64
	ChunkCount int
}

// IndexPDF indexes a PDF file end-to-end with optional metadata overrides.
func IndexPDF(ctx context.Context, db *sql.DB, embedClient *embeddings.Client, pdfPath string, opts IndexOptions) (IndexResult, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	signature, err := ComputeFileSignature(pdfPath)
	if err != nil {
		return IndexResult{}, err
	}
	return IndexPDFWithSignature(ctx, db, embedClient, pdfPath, signature, opts)
}

// IndexMarkdown indexes a markdown file end-to-end.
func IndexMarkdown(ctx context.Context, db *sql.DB, embedClient *embeddings.Client, markdownPath string, opts IndexOptions) (IndexResult, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	signature, err := ComputeFileSignature(markdownPath)
	if err != nil {
		return IndexResult{}, err
	}
	return IndexMarkdownWithSignature(ctx, db, embedClient, markdownPath, signature, opts)
}

// IndexPDFWithSignature indexes a PDF using a precomputed signature.
func IndexPDFWithSignature(ctx context.Context, db *sql.DB, embedClient *embeddings.Client, pdfPath string, signature FileSignature, opts IndexOptions) (IndexResult, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	metadata, err := convert.ExtractPDFMetadata(ctx, pdfPath)
	if err != nil {
		return IndexResult{}, fmt.Errorf("extract metadata for %s: %w", pdfPath, err)
	}
	metadata.FileModTime = signature.ModTime
	metadata.FileHash = signature.Hash
	applyMetadataOverrides(&metadata, opts.MetadataOverrides)

	markdownPath, err := convert.PDFToMarkdown(ctx, pdfPath, opts.MarkdownOptions)
	if err != nil {
		return IndexResult{}, fmt.Errorf("convert pdf to markdown for %s: %w", pdfPath, err)
	}

	markdownBytes, err := os.ReadFile(markdownPath)
	if err != nil {
		return IndexResult{}, fmt.Errorf("read markdown for %s: %w", markdownPath, err)
	}

	chunks, vectors, err := embedMarkdown(ctx, embedClient, string(markdownBytes), opts)
	if err != nil {
		return IndexResult{}, fmt.Errorf("embed chunks for %s: %w", pdfPath, err)
	}
	chunkCount := len(chunks)

	docID, err := insertDocumentTransaction(ctx, db, metadata, markdownPath, chunks, vectors)
	if err != nil {
		return IndexResult{}, fmt.Errorf("persist document for %s: %w", pdfPath, err)
	}

	return IndexResult{DocumentID: docID, ChunkCount: chunkCount}, nil
}

// IndexMarkdownWithSignature indexes a markdown file using a precomputed signature.
func IndexMarkdownWithSignature(ctx context.Context, db *sql.DB, embedClient *embeddings.Client, markdownPath string, signature FileSignature, opts IndexOptions) (IndexResult, error) {
	if ctx == nil {
		ctx = context.Background()
	}

	info, err := os.Stat(markdownPath)
	if err != nil {
		return IndexResult{}, fmt.Errorf("stat markdown for %s: %w", markdownPath, err)
	}

	metadata := convert.DocumentMetadata{
		Filename:    filepath.Base(markdownPath),
		FileSize:    info.Size(),
		PageCount:   0,
		FileModTime: signature.ModTime,
		FileHash:    signature.Hash,
	}
	applyMetadataOverrides(&metadata, opts.MetadataOverrides)

	markdownBytes, err := os.ReadFile(markdownPath)
	if err != nil {
		return IndexResult{}, fmt.Errorf("read markdown for %s: %w", markdownPath, err)
	}
	if metadata.Title == nil {
		if title := inferMarkdownTitle(string(markdownBytes)); title != nil {
			metadata.Title = title
		}
	}

	chunks, vectors, err := embedMarkdown(ctx, embedClient, string(markdownBytes), opts)
	if err != nil {
		return IndexResult{}, fmt.Errorf("embed chunks for %s: %w", markdownPath, err)
	}
	chunkCount := len(chunks)

	docID, err := insertDocumentTransaction(ctx, db, metadata, markdownPath, chunks, vectors)
	if err != nil {
		return IndexResult{}, fmt.Errorf("persist document for %s: %w", markdownPath, err)
	}

	return IndexResult{DocumentID: docID, ChunkCount: chunkCount}, nil
}

// ComputeFileSignature returns file mod time + hash for change detection.
func ComputeFileSignature(pdfPath string) (FileSignature, error) {
	info, err := os.Stat(pdfPath)
	if err != nil {
		return FileSignature{}, fmt.Errorf("stat pdf for signature: %w", err)
	}
	hash, err := md5File(pdfPath)
	if err != nil {
		return FileSignature{}, err
	}
	return FileSignature{ModTime: info.ModTime(), Hash: hash}, nil
}

func md5File(path string) (string, error) {
	file, err := os.Open(path)
	if err != nil {
		return "", fmt.Errorf("open pdf for hashing: %w", err)
	}
	defer func() {
		if closeErr := file.Close(); closeErr != nil {
			logging.L().Warn("failed to close pdf after hashing", zap.String("path", path), zap.Error(closeErr))
		}
	}()
	hasher := md5.New()
	if _, err := io.Copy(hasher, file); err != nil {
		return "", fmt.Errorf("hash pdf: %w", err)
	}
	return fmt.Sprintf("%x", hasher.Sum(nil)), nil
}

func (o IndexOptions) batchSize() int {
	return o.EmbeddingBatchSize
}

func (o IndexOptions) acquireEmbedLimiter(ctx context.Context) error {
	if o.EmbedLimiter == nil {
		return nil
	}
	select {
	case o.EmbedLimiter <- struct{}{}:
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (o IndexOptions) releaseEmbedLimiter() {
	if o.EmbedLimiter == nil {
		return
	}
	select {
	case <-o.EmbedLimiter:
	default:
	}
}

func applyMetadataOverrides(metadata *convert.DocumentMetadata, overrides MetadataOverrides) {
	if metadata == nil {
		return
	}
	if metadata.Title == nil || strings.TrimSpace(*metadata.Title) == "" {
		if value := trimmedString(overrides.Title); value != nil {
			metadata.Title = value
		}
	}
	if metadata.Authors == nil || strings.TrimSpace(*metadata.Authors) == "" {
		if value := trimmedString(overrides.Authors); value != nil {
			metadata.Authors = value
		}
	}
	if metadata.PublicationDate == nil || strings.TrimSpace(*metadata.PublicationDate) == "" {
		if value := trimmedString(overrides.PublicationDate); value != nil {
			metadata.PublicationDate = value
		}
	}
	if metadata.DOI == nil || strings.TrimSpace(*metadata.DOI) == "" {
		if value := trimmedString(overrides.DOI); value != nil {
			metadata.DOI = value
		}
	}
}

func trimmedString(value *string) *string {
	if value == nil {
		return nil
	}
	trimmed := strings.TrimSpace(*value)
	if trimmed == "" {
		return nil
	}
	copy := trimmed
	return &copy
}

func embedMarkdown(ctx context.Context, embedClient *embeddings.Client, markdown string, opts IndexOptions) ([]chunking.Chunk, [][]float32, error) {
	type attempt struct {
		opts      chunking.Options
		charLimit int
	}

	attempts := []attempt{
		{opts: chunking.DefaultOptions()},
		{
			opts: chunking.Options{
				TargetTokens:  300,
				MaxTokens:     400,
				OverlapTokens: 40,
			},
			charLimit: 2000,
		},
		{
			opts: chunking.Options{
				TargetTokens:  200,
				MaxTokens:     260,
				OverlapTokens: 30,
			},
			charLimit: 1200,
		},
		{
			opts: chunking.Options{
				TargetTokens:  120,
				MaxTokens:     160,
				OverlapTokens: 20,
			},
			charLimit: 800,
		},
		{
			opts: chunking.Options{
				TargetTokens:  80,
				MaxTokens:     120,
				OverlapTokens: 20,
			},
			charLimit: 500,
		},
	}

	var lastErr error
	for i, attempt := range attempts {
		chunks := chunking.ChunkMarkdown(0, markdown, attempt.opts)
		if attempt.charLimit > 0 {
			chunks = splitChunksByCharLimit(chunks, attempt.charLimit)
		}
		vectors, err := embedChunks(ctx, embedClient, chunks, opts)
		if err == nil {
			return chunks, vectors, nil
		}
		if !isContextLengthError(err) {
			return nil, nil, err
		}
		lastErr = err
		logging.L().Warn("embedding context length exceeded; rechunking",
			zap.Int("attempt", i+1),
			zap.Int("chunk_count", len(chunks)),
			zap.Int("max_tokens", attempt.opts.MaxTokens),
			zap.Int("char_limit", attempt.charLimit),
		)
	}
	return nil, nil, lastErr
}

func embedChunks(ctx context.Context, embedClient *embeddings.Client, chunks []chunking.Chunk, opts IndexOptions) ([][]float32, error) {
	if len(chunks) == 0 {
		return nil, nil
	}
	texts := make([]string, len(chunks))
	for i, chunk := range chunks {
		texts[i] = chunk.Content
	}
	if err := opts.acquireEmbedLimiter(ctx); err != nil {
		return nil, err
	}
	vectors, err := embedClient.EmbedTexts(ctx, texts, opts.batchSize())
	opts.releaseEmbedLimiter()
	if err != nil {
		return nil, err
	}
	if len(vectors) != len(chunks) {
		return nil, fmt.Errorf("embedding count mismatch: got %d want %d", len(vectors), len(chunks))
	}
	return vectors, nil
}

func isContextLengthError(err error) bool {
	if err == nil {
		return false
	}
	return strings.Contains(strings.ToLower(err.Error()), "context length")
}

func splitChunksByCharLimit(chunks []chunking.Chunk, limit int) []chunking.Chunk {
	if limit <= 0 {
		return chunks
	}
	out := make([]chunking.Chunk, 0, len(chunks))
	for _, chunk := range chunks {
		content := strings.TrimSpace(chunk.Content)
		if content == "" {
			continue
		}
		if len([]rune(content)) <= limit {
			out = append(out, chunk)
			continue
		}
		runes := []rune(content)
		for start := 0; start < len(runes); start += limit {
			end := start + limit
			if end > len(runes) {
				end = len(runes)
			}
			part := strings.TrimSpace(string(runes[start:end]))
			if part == "" {
				continue
			}
			frag := chunk
			frag.Content = part
			frag.TokenCount = len(strings.Fields(part))
			out = append(out, frag)
		}
	}
	return out
}

func inferMarkdownTitle(content string) *string {
	for _, line := range strings.Split(content, "\n") {
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "# ") {
			title := strings.TrimSpace(strings.TrimPrefix(trimmed, "# "))
			if title == "" {
				return nil
			}
			return &title
		}
	}
	return nil
}

func insertDocumentTransaction(ctx context.Context, db *sql.DB, metadata convert.DocumentMetadata, markdownPath string, chunks []chunking.Chunk, vectors [][]float32) (int64, error) {
	if len(vectors) > 0 && len(vectors) != len(chunks) {
		return 0, fmt.Errorf("embedding count mismatch: got %d want %d", len(vectors), len(chunks))
	}

	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		return 0, err
	}

	pubDate := parsePublicationDate(metadata.PublicationDate)
	docID, err := insertDocument(ctx, tx, metadata, markdownPath, pubDate)
	if err != nil {
		return 0, rollback(tx, err)
	}

	for i, chunk := range chunks {
		chunkID, err := insertChunk(ctx, tx, docID, i, chunk)
		if err != nil {
			return 0, rollback(tx, err)
		}
		if len(vectors) > 0 {
			if err := insertEmbedding(ctx, tx, chunkID, vectors[i]); err != nil {
				return 0, rollback(tx, err)
			}
		}
	}
	if len(vectors) > 0 {
		docEmbedding, err := computeDocumentEmbedding(vectors)
		if err != nil {
			return 0, rollback(tx, err)
		}
		if docEmbedding != nil {
			if err := insertDocumentEmbedding(ctx, tx, docID, docEmbedding); err != nil {
				return 0, rollback(tx, err)
			}
		}
	}

	if err := tx.Commit(); err != nil {
		return 0, err
	}
	return docID, nil
}

func insertDocument(ctx context.Context, tx *sql.Tx, metadata convert.DocumentMetadata, markdownPath string, pubDate *time.Time) (int64, error) {
	query := `INSERT INTO documents (id, filename, title, authors, publication_date, doi, markdown_path, page_count, file_size_bytes, file_mtime, file_hash)
		VALUES (nextval('documents_id_seq'), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		RETURNING id;`
	var publication any
	if pubDate != nil {
		publication = *pubDate
	}
	var fileMTime any
	if !metadata.FileModTime.IsZero() {
		fileMTime = metadata.FileModTime
	}
	var fileHash any
	if strings.TrimSpace(metadata.FileHash) != "" {
		fileHash = strings.TrimSpace(metadata.FileHash)
	}
	row := tx.QueryRowContext(ctx, query,
		metadata.Filename,
		nullString(metadata.Title),
		nullString(metadata.Authors),
		publication,
		nullString(metadata.DOI),
		markdownPath,
		metadata.PageCount,
		metadata.FileSize,
		fileMTime,
		fileHash,
	)
	var id int64
	if err := row.Scan(&id); err != nil {
		return 0, err
	}
	return id, nil
}

func insertChunk(ctx context.Context, tx *sql.Tx, documentID int64, index int, chunk chunking.Chunk) (int64, error) {
	query := `INSERT INTO chunks (id, document_id, chunk_index, content, page_number, section_title, token_count)
		VALUES (nextval('chunks_id_seq'), ?, ?, ?, ?, ?, ?)
		RETURNING id;`
	var page any
	if chunk.PageNumber > 0 {
		page = chunk.PageNumber
	}
	section := strings.TrimSpace(chunk.SectionTitle)
	row := tx.QueryRowContext(ctx, query,
		documentID,
		index,
		chunk.Content,
		page,
		nullStringValue(section),
		chunk.TokenCount,
	)
	var id int64
	if err := row.Scan(&id); err != nil {
		return 0, err
	}
	return id, nil
}

func insertEmbedding(ctx context.Context, tx *sql.Tx, chunkID int64, embedding []float32) error {
	blob := embeddings.EncodeEmbedding(embedding)
	_, err := tx.ExecContext(ctx, `INSERT INTO embeddings (chunk_id, embedding) VALUES (?, ?);`, chunkID, blob)
	return err
}

func insertDocumentEmbedding(ctx context.Context, tx *sql.Tx, documentID int64, embedding []float32) error {
	blob := embeddings.EncodeEmbedding(embedding)
	_, err := tx.ExecContext(ctx, `INSERT INTO document_embeddings (document_id, embedding) VALUES (?, ?);`, documentID, blob)
	return err
}

func computeDocumentEmbedding(vectors [][]float32) ([]float32, error) {
	if len(vectors) == 0 {
		return nil, nil
	}
	dim := len(vectors[0])
	if dim == 0 {
		return nil, fmt.Errorf("embedding dimension is zero")
	}
	sum := make([]float64, dim)
	for i, vec := range vectors {
		if len(vec) != dim {
			return nil, fmt.Errorf("embedding dimension mismatch at %d: got %d want %d", i, len(vec), dim)
		}
		for j, v := range vec {
			sum[j] += float64(v)
		}
	}
	avg := make([]float32, dim)
	scale := 1.0 / float64(len(vectors))
	var norm float64
	for i, v := range sum {
		scaled := v * scale
		norm += scaled * scaled
		avg[i] = float32(scaled)
	}
	if norm == 0 || math.IsNaN(norm) || math.IsInf(norm, 0) {
		return nil, fmt.Errorf("invalid document embedding norm")
	}
	norm = math.Sqrt(norm)
	for i := range avg {
		avg[i] = float32(float64(avg[i]) / norm)
	}
	return avg, nil
}

func parsePublicationDate(value *string) *time.Time {
	if value == nil {
		return nil
	}
	trimmed := strings.TrimSpace(*value)
	if trimmed == "" {
		return nil
	}
	if strings.HasPrefix(trimmed, "D:") {
		trimmed = strings.TrimPrefix(trimmed, "D:")
	}
	if parsed := parseDateDigits(trimmed); parsed != nil {
		return parsed
	}
	for _, layout := range []string{"2006-01-02", "2006/01/02", "2006.01.02", "20060102"} {
		if t, err := time.Parse(layout, trimmed); err == nil {
			return &t
		}
	}
	return nil
}

func parseDateDigits(value string) *time.Time {
	digits := make([]rune, 0, 8)
	for _, r := range value {
		if r < '0' || r > '9' {
			continue
		}
		digits = append(digits, r)
		if len(digits) == 8 {
			break
		}
	}
	if len(digits) < 8 {
		return nil
	}
	if t, err := time.Parse("20060102", string(digits[:8])); err == nil {
		return &t
	}
	return nil
}

func nullString(value *string) any {
	if value == nil {
		return nil
	}
	trimmed := strings.TrimSpace(*value)
	if trimmed == "" {
		return nil
	}
	return trimmed
}

func nullStringValue(value string) any {
	if strings.TrimSpace(value) == "" {
		return nil
	}
	return strings.TrimSpace(value)
}

func rollback(tx *sql.Tx, err error) error {
	if rbErr := tx.Rollback(); rbErr != nil && rbErr != sql.ErrTxDone {
		return fmt.Errorf("%w: %v", err, rbErr)
	}
	return err
}
