//! @efficiency-role: domain-logic
//!
//! Hybrid Search Memory System (Task 273)
//!
//! Combines full-text search (FTS) with lightweight vector-like similarity
//! for improved knowledge retrieval. Inspired by OpenCrabs' hybrid search.
//!
//! Implementation uses:
//! - TF-IDF scoring for full-text matching
//! - Keyword overlap and position scoring for semantic similarity
//! - Hybrid ranking that combines both signals

use crate::*;
use std::collections::{HashMap, HashSet};

// ============================================================================
// Search Types
// ============================================================================

/// A searchable memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MemoryEntry {
    /// Unique identifier
    pub id: String,
    /// Entry content/text
    pub content: String,
    /// Optional metadata tags
    pub tags: Vec<String>,
    /// Optional category
    pub category: Option<String>,
    /// Creation timestamp
    pub created_at: u64,
    /// Access count (for relevance boosting)
    pub access_count: usize,
}

impl MemoryEntry {
    pub fn new(id: &str, content: &str) -> Self {
        Self {
            id: id.to_string(),
            content: content.to_string(),
            tags: Vec::new(),
            category: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            access_count: 0,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_category(mut self, category: &str) -> Self {
        self.category = Some(category.to_string());
        self
    }
}

/// Search result with combined scores
#[derive(Debug, Clone)]
pub(crate) struct SearchResult {
    /// The matching entry
    pub entry: MemoryEntry,
    /// FTS score (0.0-1.0)
    pub fts_score: f64,
    /// Vector-like similarity score (0.0-1.0)
    pub vector_score: f64,
    /// Combined hybrid score (0.0-1.0)
    pub hybrid_score: f64,
}

/// Search mode for hybrid queries
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SearchMode {
    /// Full-text search only
    FTS,
    /// Vector-like similarity only
    Vector,
    /// Hybrid combination (default)
    Hybrid,
}

impl Default for SearchMode {
    fn default() -> Self {
        SearchMode::Hybrid
    }
}

// ============================================================================
// Hybrid Search Engine
// ============================================================================

/// Inverted index for FTS
struct InvertedIndex {
    /// term -> list of entry ids
    postings: HashMap<String, Vec<String>>,
    /// entry id -> term frequency map
    term_freqs: HashMap<String, HashMap<String, usize>>,
    /// Total number of documents
    doc_count: usize,
}

impl InvertedIndex {
    fn new() -> Self {
        Self {
            postings: HashMap::new(),
            term_freqs: HashMap::new(),
            doc_count: 0,
        }
    }

    /// Add a document to the index
    fn add(&mut self, entry_id: &str, content: &str) {
        let terms = tokenize(content);
        let mut term_freq = HashMap::new();

        for term in &terms {
            *term_freq.entry(term.clone()).or_insert(0) += 1;
            self.postings
                .entry(term.clone())
                .or_insert_with(Vec::new)
                .push(entry_id.to_string());
        }

        self.term_freqs.insert(entry_id.to_string(), term_freq);
        self.doc_count += 1;
    }

    /// Search for documents containing query terms
    fn search(&self, query: &str) -> HashMap<String, f64> {
        let query_terms = tokenize(query);
        if query_terms.is_empty() || self.doc_count == 0 {
            return HashMap::new();
        }

        let mut scores: HashMap<String, f64> = HashMap::new();
        let total_docs = self.doc_count as f64;

        for term in &query_terms {
            if let Some(doc_ids) = self.postings.get(term) {
                let df = doc_ids.len() as f64;
                // IDF = log(N / df)
                let idf = (total_docs / df.max(1.0)).ln().max(0.0);

                for doc_id in doc_ids {
                    if let Some(tf_map) = self.term_freqs.get(doc_id) {
                        let tf = tf_map.get(term).copied().unwrap_or(0) as f64;
                        // TF-IDF score
                        let score = tf * idf;
                        *scores.entry(doc_id.clone()).or_insert(0.0) += score;
                    }
                }
            }
        }

        // Normalize scores to 0-1 range
        let max_score = scores.values().cloned().fold(0.0_f64, f64::max);
        if max_score > 0.0 {
            for score in scores.values_mut() {
                *score /= max_score;
            }
        }

        scores
    }
}

/// Tokenize text into lowercase terms
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 2) // Skip very short terms
        .map(|s| s.to_string())
        .collect()
}

/// Compute keyword overlap similarity (vector-like scoring)
fn keyword_similarity(query: &str, content: &str) -> f64 {
    let query_terms: HashSet<String> = tokenize(query).into_iter().collect();
    let content_terms: HashSet<String> = tokenize(content).into_iter().collect();

    if query_terms.is_empty() || content_terms.is_empty() {
        return 0.0;
    }

    // Jaccard similarity
    let intersection: usize = query_terms.intersection(&content_terms).count();
    let union: usize = query_terms.union(&content_terms).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Compute position-aware similarity (bonus for term proximity)
fn position_similarity(query: &str, content: &str) -> f64 {
    let query_terms = tokenize(query);
    let content_terms = tokenize(content);

    if query_terms.is_empty() || content_terms.is_empty() {
        return 0.0;
    }

    // Find positions of query terms in content
    let mut positions: Vec<usize> = Vec::new();
    for (i, term) in content_terms.iter().enumerate() {
        if query_terms.contains(term) {
            positions.push(i);
        }
    }

    if positions.len() < 2 {
        return positions.len() as f64 / query_terms.len().max(1) as f64;
    }

    // Calculate average gap between consecutive matches
    let mut total_gap = 0;
    for i in 1..positions.len() {
        total_gap += positions[i] - positions[i - 1];
    }
    let avg_gap = total_gap as f64 / (positions.len() - 1) as f64;

    // Closer terms = higher score (inverse of avg gap)
    let proximity_score = 1.0 / (1.0 + avg_gap / 5.0); // Normalize with factor of 5

    // Combine with coverage
    let coverage = positions.len() as f64 / query_terms.len().max(1) as f64;
    (proximity_score + coverage) / 2.0
}

/// Hybrid search engine
pub(crate) struct HybridSearchEngine {
    /// Memory entries
    entries: HashMap<String, MemoryEntry>,
    /// FTS index
    fts_index: InvertedIndex,
    /// FTS weight in hybrid scoring (0.0-1.0)
    fts_weight: f64,
    /// Vector weight in hybrid scoring (0.0-1.0)
    vector_weight: f64,
}

impl HybridSearchEngine {
    /// Create a new search engine
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            fts_index: InvertedIndex::new(),
            fts_weight: 0.6, // FTS slightly preferred for exact matching
            vector_weight: 0.4,
        }
    }

    /// Create with custom weights
    pub fn with_weights(fts_weight: f64, vector_weight: f64) -> Self {
        let total = fts_weight + vector_weight;
        Self {
            entries: HashMap::new(),
            fts_index: InvertedIndex::new(),
            fts_weight: fts_weight / total,
            vector_weight: vector_weight / total,
        }
    }

    /// Add an entry to the search index
    pub fn add_entry(&mut self, entry: MemoryEntry) {
        let id = entry.id.clone();
        let content = entry.content.clone();
        self.fts_index.add(&id, &content);
        self.entries.insert(id, entry);
    }

    /// Remove an entry by ID
    pub fn remove_entry(&mut self, id: &str) -> Option<MemoryEntry> {
        self.entries.remove(id)
    }

    /// Search for entries matching the query
    pub fn search(&self, query: &str, mode: SearchMode, max_results: usize) -> Vec<SearchResult> {
        if self.entries.is_empty() || query.trim().is_empty() {
            return Vec::new();
        }

        let fts_scores = match mode {
            SearchMode::FTS | SearchMode::Hybrid => self.fts_index.search(query),
            SearchMode::Vector => HashMap::new(),
        };

        let mut results: Vec<SearchResult> = Vec::new();

        for (id, entry) in &self.entries {
            let fts_score = fts_scores.get(id).copied().unwrap_or(0.0);

            let vector_score = match mode {
                SearchMode::Vector | SearchMode::Hybrid => {
                    let keyword_sim = keyword_similarity(query, &entry.content);
                    let position_sim = position_similarity(query, &entry.content);
                    (keyword_sim + position_sim) / 2.0
                }
                SearchMode::FTS => 0.0,
            };

            let hybrid_score = match mode {
                SearchMode::Hybrid => {
                    self.fts_weight * fts_score + self.vector_weight * vector_score
                }
                SearchMode::FTS => fts_score,
                SearchMode::Vector => vector_score,
            };

            if hybrid_score > 0.0 {
                results.push(SearchResult {
                    entry: entry.clone(),
                    fts_score,
                    vector_score,
                    hybrid_score,
                });
            }
        }

        // Sort by hybrid score descending
        results.sort_by(|a, b| {
            b.hybrid_score
                .partial_cmp(&a.hybrid_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply access count boost (recently accessed items get slight boost)
        for result in &mut results {
            let access_boost = (result.entry.access_count as f64 * 0.01).min(0.1);
            result.hybrid_score = (result.hybrid_score + access_boost).min(1.0);
        }

        // Re-sort after boost
        results.sort_by(|a, b| {
            b.hybrid_score
                .partial_cmp(&a.hybrid_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(max_results);

        results
    }

    /// Increment access count for an entry
    pub fn record_access(&mut self, id: &str) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.access_count += 1;
        }
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get all entries
    pub fn get_all_entries(&self) -> Vec<&MemoryEntry> {
        self.entries.values().collect()
    }

    /// Get entry by ID
    pub fn get_entry(&self, id: &str) -> Option<&MemoryEntry> {
        self.entries.get(id)
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.fts_index = InvertedIndex::new();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_engine() -> HybridSearchEngine {
        let mut engine = HybridSearchEngine::new();

        engine.add_entry(MemoryEntry::new(
            "1",
            "Rust is a systems programming language focused on safety and performance",
        ));
        engine.add_entry(MemoryEntry::new(
            "2",
            "Python is great for data science and machine learning",
        ));
        engine.add_entry(MemoryEntry::new(
            "3",
            "JavaScript runs in web browsers and Node.js",
        ));
        engine.add_entry(MemoryEntry::new(
            "4",
            "Rust memory safety prevents buffer overflows and data races",
        ));
        engine.add_entry(MemoryEntry::new(
            "5",
            "Python web frameworks include Django and Flask",
        ));

        engine
    }

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Hello World Test");
        assert_eq!(tokens.len(), 3);
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
    }

    #[test]
    fn test_tokenize_filters_short() {
        let tokens = tokenize("a an be word");
        assert!(!tokens.contains(&"a".to_string()));
        assert!(!tokens.contains(&"an".to_string()));
        assert!(!tokens.contains(&"be".to_string()));
        assert!(tokens.contains(&"word".to_string()));
    }

    #[test]
    fn test_keyword_similarity() {
        let sim = keyword_similarity("rust programming", "rust is a programming language");
        assert!(sim > 0.0);
        assert!(sim <= 1.0);
    }

    #[test]
    fn test_keyword_similarity_no_overlap() {
        let sim = keyword_similarity("rust programming", "python data science");
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_hybrid_search_fts_mode() {
        let engine = create_test_engine();
        let results = engine.search("rust safety", SearchMode::FTS, 5);
        assert!(!results.is_empty());
        assert!(results[0].entry.id == "1" || results[0].entry.id == "4");
    }

    #[test]
    fn test_hybrid_search_vector_mode() {
        let engine = create_test_engine();
        let results = engine.search("rust safety", SearchMode::Vector, 5);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_hybrid_search_hybrid_mode() {
        let engine = create_test_engine();
        let results = engine.search("rust safety", SearchMode::Hybrid, 5);
        assert!(!results.is_empty());
        // Hybrid should find Rust-related entries
        assert!(results
            .iter()
            .any(|r| r.entry.id == "1" || r.entry.id == "4"));
    }

    #[test]
    fn test_search_returns_empty_for_no_matches() {
        let engine = create_test_engine();
        let results = engine.search("kubernetes docker", SearchMode::Hybrid, 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_respects_max_results() {
        let engine = create_test_engine();
        let results = engine.search("rust", SearchMode::Hybrid, 1);
        assert!(results.len() <= 1);
    }

    #[test]
    fn test_entry_count() {
        let engine = create_test_engine();
        assert_eq!(engine.entry_count(), 5);
    }

    #[test]
    fn test_remove_entry() {
        let mut engine = create_test_engine();
        let removed = engine.remove_entry("1");
        assert!(removed.is_some());
        assert_eq!(engine.entry_count(), 4);
        assert!(engine.get_entry("1").is_none());
    }

    #[test]
    fn test_clear_engine() {
        let mut engine = create_test_engine();
        engine.clear();
        assert_eq!(engine.entry_count(), 0);
    }

    #[test]
    fn test_custom_weights() {
        let engine = HybridSearchEngine::with_weights(0.8, 0.2);
        assert!((engine.fts_weight - 0.8).abs() < 0.01);
        assert!((engine.vector_weight - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_search_result_scores() {
        let engine = create_test_engine();
        let results = engine.search("rust", SearchMode::Hybrid, 5);

        for result in &results {
            assert!(result.hybrid_score >= 0.0);
            assert!(result.hybrid_score <= 1.0);
            assert!(result.fts_score >= 0.0);
            assert!(result.vector_score >= 0.0);
        }
    }

    #[test]
    fn test_access_count_boost() {
        let mut engine = create_test_engine();

        // Search and record access multiple times for "rust"
        for _ in 0..5 {
            let results = engine.search("rust", SearchMode::Hybrid, 5);
            for result in &results {
                engine.record_access(&result.entry.id);
            }
        }

        // Entries should have increased access counts
        let rust_entries: Vec<_> = engine
            .get_all_entries()
            .into_iter()
            .filter(|e| e.content.contains("Rust"))
            .collect();

        for entry in rust_entries {
            assert!(entry.access_count > 0);
        }
    }

    #[test]
    fn test_memory_entry_with_tags() {
        let entry = MemoryEntry::new("1", "test content")
            .with_tags(vec!["rust".to_string(), "programming".to_string()])
            .with_category("language");

        assert_eq!(entry.tags.len(), 2);
        assert_eq!(entry.category, Some("language".to_string()));
    }
}
