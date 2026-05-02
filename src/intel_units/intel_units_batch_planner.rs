use crate::intel_trait::{IntelContext, IntelOutput, IntelUnit};
use crate::repo_map::RepoMapCache;
use crate::*;

pub(crate) use crate::BatchableItem;
pub(crate) use crate::BatchPlannerInput;
pub(crate) use crate::BatchPlan;
pub(crate) use crate::BatchGroup;
pub(crate) use crate::ItemKind;

pub(crate) struct BatchPlannerUnit;

impl IntelUnit for BatchPlannerUnit {
    fn name(&self) -> &'static str { "batch_planner" }
    fn profile(&self) -> &Profile {
        static DUMMY: once_cell::sync::OnceCell<Profile> = once_cell::sync::OnceCell::new();
        DUMMY.get_or_init(|| Profile {
            version: 1,
            name: "batch_planner".to_string(),
            base_url: String::new(),
            model: "deterministic".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 32,
            timeout_s: 3,
            system_prompt: String::new(),
        })
    }
    async fn execute(&self, ctx: &IntelContext) -> Result<IntelOutput> {
        let input: BatchPlannerInput = serde_json::from_value(
            ctx.extra("input").cloned().unwrap_or(serde_json::Value::Null)
        )
            .map_err(|e| IntelError::ParseError("batch planner input".to_string(), e.to_string()))?;

        let plan = Self::plan_batches(&input);
        let data = serde_json::to_value(&plan)
            .map_err(|e| IntelError::ParseError("batch plan serialization".to_string(), e.to_string()))?;

        Ok(IntelOutput {
            unit_name: self.name().to_string(),
            data,
            confidence: 1.0,
            fallback_used: false,
            fallback_reason: None,
        })
    }
}

impl BatchPlannerUnit {
    pub fn plan_batches(input: &BatchPlannerInput) -> BatchPlan {
        let mut items: Vec<_> = input.items.clone();
        items.sort_by(|a, b| b.estimated_tokens.cmp(&a.estimated_tokens));

        let effective_budget = input.available_budget_per_batch
            .saturating_sub(input.response_buffer_tokens);

        let mut batches: Vec<BatchGroup> = Vec::new();
        let mut current_uris: Vec<String> = Vec::new();
        let mut current_kinds: Vec<ItemKind> = Vec::new();
        let mut current_tokens: usize = 0;

        for item in &items {
            let would_overflow = current_tokens + item.estimated_tokens > effective_budget;
            let at_max_items = current_uris.len() >= input.max_items_per_batch;

            if (would_overflow || at_max_items) && !current_uris.is_empty() {
                let batch_num = batches.len() + 1;
                batches.push(BatchGroup {
                    batch_number: batch_num,
                    item_uris: std::mem::take(&mut current_uris),
                    item_kinds: std::mem::take(&mut current_kinds),
                    estimated_tokens: current_tokens,
                    summary_prompt: Self::build_summary_prompt(
                        batch_num, &input.objective, batch_num > 1
                    ),
                    depends_on_previous: batch_num > 1,
                });
                current_tokens = 0;
            }

            current_uris.push(item.source_kind.to_uri());
            current_kinds.push(item.source_kind.clone());
            current_tokens += item.estimated_tokens;
        }

        if !current_uris.is_empty() {
            let batch_num = batches.len() + 1;
            batches.push(BatchGroup {
                batch_number: batch_num,
                item_uris: current_uris,
                item_kinds: current_kinds,
                estimated_tokens: current_tokens,
                summary_prompt: Self::build_summary_prompt(
                    batch_num, &input.objective, batch_num > 1
                ),
                depends_on_previous: batch_num > 1,
            });
        }

        let total_items: usize = batches.iter().map(|b| b.item_uris.len()).sum();
        let total_tokens: usize = batches.iter().map(|b| b.estimated_tokens).sum();
        let batch_count = batches.len();
        let estimated_cost = total_tokens + batch_count * input.response_buffer_tokens;

        BatchPlan {
            batches,
            total_items,
            total_estimated_tokens: total_tokens,
            batch_count,
            estimated_total_cost_tokens: estimated_cost,
        }
    }

    fn build_summary_prompt(batch_num: usize, objective: &str, include_prior: bool) -> String {
        let base = format!(
            "Batch {}: Analyze the content from these items focusing on their relevance \
             to the objective: \"{}\". Identify key structures, functions, types, and \
             patterns. Note connections between items in this batch. Be thorough — this \
             summary may be the only representation of these items in later analysis.",
            batch_num, objective
        );
        if include_prior {
            format!(
                "{}\n\nCross-reference findings with previous batch summaries. \
                 Note confirmations, contradictions, and new insights. \
                 Build cumulative understanding toward the objective.",
                base
            )
        } else {
            base
        }
    }

    pub fn apply_semantic_grouping(
        plan: &mut BatchPlan,
        _repo_map: &RepoMapCache,
    ) {
        if plan.batches.len() <= 1 {
            return;
        }
        for batch in &mut plan.batches {
            let mut paired: Vec<(usize, String, ItemKind)> = batch.item_uris.iter()
                .zip(batch.item_kinds.iter())
                .enumerate()
                .map(|(pos, (uri, kind))| (pos, uri.clone(), kind.clone()))
                .collect();

            paired.sort_by(|(pos_a, uri_a, kind_a), (pos_b, uri_b, kind_b)| {
                let a_is_file = matches!(kind_a, ItemKind::FilePath(_));
                let b_is_file = matches!(kind_b, ItemKind::FilePath(_));
                if a_is_file && b_is_file {
                    let dir_a = std::path::Path::new(uri_a).parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let dir_b = std::path::Path::new(uri_b).parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    dir_a.cmp(&dir_b).then_with(|| uri_a.cmp(uri_b))
                } else {
                    pos_a.cmp(pos_b)
                }
            });

            batch.item_uris = paired.iter().map(|(_, uri, _)| uri.clone()).collect();
            batch.item_kinds = paired.iter().map(|(_, _, kind)| kind.clone()).collect();
        }
    }
}
