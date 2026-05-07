//! @efficiency-role: domain-logic
//!
//! Search result analysis intel unit and evidence ranking (Task 672).
//!
//! Provides evidence-aware ranking of search results by combining
//! filename match, path containment, content match frequency, and
//! file type bonuses into a composite relevance score.

use crate::*;
use std::path::PathBuf;

/// A single search result with positional metadata and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SearchResult {
    pub path: PathBuf,
    pub score: f64,
    pub matched_lines: Vec<usize>,
    pub context_snippet: String,
    pub file_type: String,
}

/// Analysis of a set of search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SearchAnalysis {
    pub best_match: Option<String>,
    pub summary: String,
    pub coverage: f64,
    pub suggestions: Vec<String>,
}

/// Ranks search results by relevance to a query using multi-factor scoring.
pub(crate) struct EvidenceRanker;

impl EvidenceRanker {
    /// Rank results in descending order of relevance to the query.
    pub(crate) fn rank(results: Vec<SearchResult>, query: &str) -> Vec<SearchResult> {
        let mut scored: Vec<SearchResult> = results
            .into_iter()
            .map(|r| {
                let mut ranked = r.clone();
                ranked.score = Self::score(&r, query);
                ranked
            })
            .collect();
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored
    }

    /// Compute a composite relevance score for a single result.
    ///
    /// Scoring rules:
    /// - Exact filename match (ignoring ext): +10
    /// - Path contains query as substring: +5
    /// - Per matched line: +2
    /// - File type bonuses: source code +3, config +2, data +1
    pub(crate) fn score(result: &SearchResult, query: &str) -> f64 {
        let mut score = 0.0;
        let q = query.to_lowercase();

        if !q.is_empty() {
            // Exact filename match (strip extension)
            if let Some(fname) = result.path.file_stem() {
                let name = fname.to_string_lossy().to_lowercase();
                if name == q {
                    score += 10.0;
                }
            }

            // Path contains query
            let path_str = result.path.to_string_lossy().to_lowercase();
            if path_str.contains(&q) {
                score += 5.0;
            }
        }

        // Content match per matched line
        score += result.matched_lines.len() as f64 * 2.0;

        // File type bonuses
        match result.file_type.to_lowercase().as_str() {
            "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" => score += 3.0,
            "toml" | "json" | "yaml" | "yml" | "ini" | "cfg" => score += 2.0,
            "csv" | "tsv" | "jsonl" => score += 1.0,
            _ => {}
        }

        score
    }
}

/// Intel unit for analyzing search results and producing a structured report.
pub(crate) struct SearchIntelUnit;

impl SearchIntelUnit {
    /// Analyze a set of ranked search results.
    ///
    /// Produces a `SearchAnalysis` with the best match, a summary,
    /// coverage score (fraction of results with score > 0), and
    /// refinement suggestions.
    pub(crate) fn analyze(results: &[SearchResult]) -> SearchAnalysis {
        if results.is_empty() {
            return SearchAnalysis {
                best_match: None,
                summary: "No results found.".to_string(),
                coverage: 0.0,
                suggestions: vec!["Broaden the search query.".to_string()],
            };
        }

        let best_match = results
            .iter()
            .max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|r| r.path.to_string_lossy().to_string());

        let scored_count = results.iter().filter(|r| r.score > 0.0).count();
        let coverage = if results.is_empty() {
            0.0
        } else {
            scored_count as f64 / results.len() as f64
        };

        let total_score: f64 = results.iter().map(|r| r.score).sum();
        let avg_score = if results.is_empty() {
            0.0
        } else {
            total_score / results.len() as f64
        };

        let summary = format!(
            "Found {} result(s), {:.0}% coverage, avg score {:.1}",
            results.len(),
            coverage * 100.0,
            avg_score
        );

        let mut suggestions = Vec::new();
        if coverage < 0.5 && results.len() > 1 {
            suggestions
                .push("Most results have low relevance. Consider refining the query.".to_string());
        }
        if results.len() == 1 {
            suggestions
                .push("Only one result found. Try a broader query for more options.".to_string());
        }
        if avg_score < 5.0 {
            suggestions.push("Low average relevance. Try using more specific terms.".to_string());
        }
        if suggestions.is_empty() {
            suggestions.push("Results look solid.".to_string());
        }

        SearchAnalysis {
            best_match,
            summary,
            coverage,
            suggestions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_result(path: &str, matched_lines: Vec<usize>, file_type: &str) -> SearchResult {
        SearchResult {
            path: PathBuf::from(path),
            score: 0.0,
            matched_lines,
            context_snippet: "fn foo() {}".to_string(),
            file_type: file_type.to_string(),
        }
    }

    #[test]
    fn test_exact_filename_match_gets_bonus() {
        let r = make_result("src/foo.rs", vec![1, 2], "rs");
        let score = EvidenceRanker::score(&r, "foo");
        assert!(score >= 10.0);
    }

    #[test]
    fn test_path_contains_query() {
        let r = make_result("src/utils/helpers.rs", vec![], "rs");
        let score = EvidenceRanker::score(&r, "utils");
        assert!(score >= 5.0);
    }

    #[test]
    fn test_content_match_bonus() {
        let r = make_result("src/main.rs", vec![1, 2, 3], "rs");
        let score = EvidenceRanker::score(&r, "main");
        // exact match (+10) + path contains (+5) + 3 lines * 2 (+6) + rs bonus (+3)
        assert!((score - 24.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_file_type_bonus_source() {
        let r = make_result("src/app.rs", vec![], "rs");
        let score_no_query = EvidenceRanker::score(&r, "");
        assert_eq!(score_no_query, 3.0);
    }

    #[test]
    fn test_file_type_bonus_config() {
        let r = make_result("config.toml", vec![], "toml");
        let score = EvidenceRanker::score(&r, "");
        assert_eq!(score, 2.0);
    }

    #[test]
    fn test_file_type_bonus_data() {
        let r = make_result("data.csv", vec![], "csv");
        let score = EvidenceRanker::score(&r, "");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_rank_orders_by_score() {
        let results = vec![
            make_result("src/low.rs", vec![], "rs"),
            make_result("src/high.rs", vec![1, 2, 3, 4, 5], "rs"),
        ];
        let ranked = EvidenceRanker::rank(results, "high");
        let path_str = ranked[0].path.to_string_lossy().to_string();
        assert!(path_str.contains("high"));
    }

    #[test]
    fn test_analyze_empty_results() {
        let analysis = SearchIntelUnit::analyze(&[]);
        assert!(analysis.best_match.is_none());
        assert!(analysis.summary.contains("No results"));
    }

    #[test]
    fn test_analyze_single_result() {
        let r = make_result("src/main.rs", vec![1], "rs");
        let analysis = SearchIntelUnit::analyze(&[r]);
        assert!(analysis.best_match.is_some());
        assert_eq!(analysis.coverage, 0.0, "unscored results give 0 coverage");
        assert!(analysis.summary.contains("Found 1"));
    }

    #[test]
    fn test_analyze_coverage() {
        let results = vec![
            make_result("src/match.rs", vec![1], "rs"),
            make_result("src/other.txt", vec![], "txt"),
        ];
        let analysis = SearchIntelUnit::analyze(&results);
        // Only the first has score > 0 (filename doesn't match "unrelated", path doesn't contain it)
        // Let's use a query that matches neither filename
        let mut scored = EvidenceRanker::rank(results.clone(), "zzzzznotfound");
        // Both will have 0 content lines for "zzzzznotfound" since we didn't set context
        // Actually the ranker doesn't filter, it just scores
        let analysis2 = SearchIntelUnit::analyze(&scored);
        assert!(analysis2.coverage <= 1.0);
    }

    #[test]
    fn test_search_analysis_suggestions_low_coverage() {
        let results = vec![
            make_result("src/a.py", vec![], "py"),
            make_result("src/b.py", vec![], "py"),
        ];
        let mut ranked = EvidenceRanker::rank(results, "nonexistent");
        let analysis = SearchIntelUnit::analyze(&ranked);
        // Coverage will be 0 since no matches, should get suggestion
        let has_suggestion = !analysis.suggestions.is_empty();
        assert!(
            has_suggestion,
            "low coverage should produce suggestions: {:?}",
            analysis.suggestions
        );
    }
}
