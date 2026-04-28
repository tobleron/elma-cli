"""LLM module using OpenAI-compatible API."""

import json
import time
import re
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, field
from openai import OpenAI, APIError, RateLimitError, APITimeoutError
import tenacity

from observability import get_logger, api_latency, api_errors, traced, log_error_alert, observe_latency


@dataclass
class GenerationResult:
    """Structured result from LLM generation."""
    answer: str
    reasoning: Optional[str] = None
    cited_sources: List[Dict[str, Any]] = field(default_factory=list)
    confidence: Optional[float] = None  # 0.0 to 1.0
    model_used: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "answer": self.answer,
            "reasoning": self.reasoning,
            "cited_sources": self.cited_sources,
            "confidence": self.confidence,
            "model_used": self.model_used
        }


# Default few-shot examples — can be overridden via constructor
DEFAULT_FEW_SHOT_EXAMPLES: List[Dict[str, Any]] = []


# Retry configuration (same defaults as embedder)
DEFAULT_MAX_ATTEMPTS = 3
DEFAULT_INITIAL_BACKOFF = 1.0
DEFAULT_MAX_BACKOFF = 10.0
DEFAULT_BACKOFF_FACTOR = 2.0


def _is_retryable_error(exception: Exception) -> bool:
    """Check if an exception is a transient error worth retrying."""
    if isinstance(exception, (RateLimitError, APITimeoutError)):
        return True
    if isinstance(exception, APIError):
        if hasattr(exception, 'status_code'):
            return 500 <= exception.status_code < 600
        return True
    err_name = type(exception).__name__.lower()
    return 'timeout' in err_name or 'connection' in err_name or 'network' in err_name


def _build_retry_callback(logger):
    """Build a callback to log retry attempts."""
    def log_retry(retry_state):
        if retry_state.outcome is None:
            return
        exception = retry_state.outcome.exception()
        wait = retry_state.next_action.sleep if retry_state.next_action else 0
        if exception:
            logger.warning(
                f"Retrying LLM API call",
                attempt=retry_state.attempt_number,
                wait_seconds=round(wait, 2),
                error_type=type(exception).__name__,
                error=str(exception)[:100]
            )
    return log_retry


class LLMAPIClient:
    """Generates answers using OpenAI-compatible API with retry support."""

    def __init__(
        self,
        api_base: str = "http://localhost:1234/v1",
        api_key: str = "not-needed",
        model: str = "llama3.2",
        max_attempts: int = DEFAULT_MAX_ATTEMPTS,
        initial_backoff: float = DEFAULT_INITIAL_BACKOFF,
        backoff_factor: float = DEFAULT_BACKOFF_FACTOR,
        max_backoff: float = DEFAULT_MAX_BACKOFF,
        few_shot_examples: Optional[List[Dict[str, Any]]] = None,
    ):
        """Initialize the LLM API client.

        Args:
            api_base: Base URL for the API.
            api_key: API key (often not needed for local servers).
            model: Model name to use.
            max_attempts: Maximum retry attempts for API calls.
            initial_backoff: Initial backoff seconds between retries.
            backoff_factor: Multiplier for backoff after each retry.
            max_backoff: Maximum backoff seconds.
            few_shot_examples: Optional list of examples for few-shot prompting.
                Each example is a dict with keys: question, context, answer.
        """
        self.api_base = api_base
        self.api_key = api_key
        self.model = model
        self.max_attempts = max_attempts
        self.initial_backoff = initial_backoff
        self.backoff_factor = backoff_factor
        self.max_backoff = max_backoff
        self.few_shot_examples = few_shot_examples or DEFAULT_FEW_SHOT_EXAMPLES
        self.client = OpenAI(base_url=api_base, api_key=api_key)
        self.logger = get_logger(__name__)

    @property
    def _retry_kwargs(self) -> dict:
        """Build tenacity retry kwargs from instance config."""
        return {
            "retry": tenacity.retry_if_exception(_is_retryable_error),
            "wait": tenacity.wait_exponential(
                multiplier=self.initial_backoff,
                exp_base=self.backoff_factor,
                max=self.max_backoff
            ),
            "stop": tenacity.stop_after_attempt(self.max_attempts),
            "before_sleep": _build_retry_callback(self.logger),
            "reraise": True,
        }

    def generate(
        self,
        question: str,
        context: Optional[List[Dict[str, Any]]] = None
    ) -> str:
        """Generate answer for a question with optional context.

        Args:
            question: User question.
            context: List of retrieved context chunks.

        Returns:
            Generated answer text.
        """
        self.logger.debug("LLM generate called",
                        has_context=context is not None,
                        context_length=len(context) if context else 0)

        start = time.perf_counter()
        try:
            if not context:
                result = self._generate_without_context(question)
            else:
                prompt = self._build_prompt(question, context)

                @tenacity.retry(**self._retry_kwargs)
                def _call():
                    return self.client.chat.completions.create(
                        model=self.model,
                        messages=[{"role": "user", "content": prompt}],
                        max_tokens=2048,
                        temperature=0.7
                    )

                response = _call()
                result = self._extract_content(response)

            duration = time.perf_counter() - start
            observe_latency(api_latency, {"client": "llm", "operation": "generate"}, duration)
            self.logger.debug("LLM generate succeeded", response_length=len(result))
            return result

        except Exception as e:
            duration = time.perf_counter() - start
            observe_latency(api_latency, {"client": "llm", "operation": "generate"}, duration)
            log_error_alert(self.logger, e, "llm_api",
                          context={"question_length": len(question)})
            raise

    def _extract_content(self, response) -> str:
        """Extract content from API response, handling various model output formats."""
        message = response.choices[0].message
        
        content = getattr(message, "content", None) or ""
        reasoning = getattr(message, "reasoning_content", None) or ""
        
        if content and content.strip():
            return content.strip()
        elif reasoning and reasoning.strip():
            return reasoning.strip()
        else:
            return "抱歉，未能生成有效回答"

    def _build_prompt(
        self,
        question: str,
        context: List[Dict[str, Any]],
        include_reasoning: bool = False
    ) -> str:
        """Build prompt with context and question.

        Args:
            question: User question.
            context: Retrieved context chunks.
            include_reasoning: If True, add instruction to include reasoning steps.

        Returns:
            Formatted prompt string.
        """
        context_texts = []
        for i, chunk in enumerate(context):
            context_texts.append(
                f"[Source {i+1}] ({chunk['source']}):\n{chunk['text']}"
            )

        context_section = "\n\n".join(context_texts)

        examples_section = ""
        if self.few_shot_examples:
            examples_section = "\n\n## Examples\n\n"
            for ex in self.few_shot_examples:
                ex_context_texts = []
                for j, ex_chunk in enumerate(ex.get("context", [])):
                    ex_context_texts.append(
                        f"[Source {j+1}] ({ex_chunk['source']}):\n{ex_chunk['text']}"
                    )
                examples_section += f"Question: {ex['question']}\n\nContext:\n" + "\n\n".join(ex_context_texts) + f"\n\nAnswer: {ex['answer']}\n\n---\n\n"
            examples_section = examples_section.rstrip()

        reasoning_instruction = (
            "\n\nFirst, explain your reasoning step by step. "
            "Then provide your final answer."
            if include_reasoning else ""
        )

        prompt = f"""Based on the following context, please answer the user's question directly and concisely.{reasoning_instruction}{examples_section}

Context:
{context_section}

Question: {question}

Answer:"""
        return prompt

    def generate_with_reasoning(
        self,
        question: str,
        context: Optional[List[Dict[str, Any]]] = None
    ) -> GenerationResult:
        """Generate answer with chain-of-thought reasoning and structured output.

        Args:
            question: User question.
            context: List of retrieved context chunks.

        Returns:
            GenerationResult with answer, reasoning, cited_sources, confidence.
        """
        self.logger.debug("LLM generate_with_reasoning called",
                        has_context=context is not None,
                        context_length=len(context) if context else 0)

        start = time.perf_counter()
        try:
            if not context:
                return GenerationResult(
                    answer="抱歉，没有提供上下文，无法回答问题。",
                    reasoning=None,
                    cited_sources=[],
                    confidence=None,
                    model_used=self.model
                )

            prompt = self._build_prompt(question, context, include_reasoning=True)

            @tenacity.retry(**self._retry_kwargs)
            def _call():
                return self.client.chat.completions.create(
                    model=self.model,
                    messages=[{"role": "user", "content": prompt}],
                    max_tokens=2048,
                    temperature=0.3  # Lower temp for more consistent reasoning
                )

            response = _call()
            raw_content = self._extract_content(response)

            result = self._parse_structured_response(raw_content, context)

            # Also try OpenAI's response_format for JSON if available
            result.model_used = self.model
            result = self._try_json_mode(question, context, result)

            duration = time.perf_counter() - start
            observe_latency(api_latency, {"client": "llm", "operation": "generate_with_reasoning"}, duration)
            self.logger.debug("LLM generate_with_reasoning succeeded",
                            has_reasoning=bool(result.reasoning),
                            num_citations=len(result.cited_sources))
            return result

        except Exception as e:
            duration = time.perf_counter() - start
            observe_latency(api_latency, {"client": "llm", "operation": "generate_with_reasoning"}, duration)
            log_error_alert(self.logger, e, "llm_api",
                          context={"question_length": len(question)})
            raise

    def _try_json_mode(
        self,
        question: str,
        context: List[Dict[str, Any]],
        fallback_result: GenerationResult
    ) -> GenerationResult:
        """Attempt structured JSON generation for more reliable parsing."""
        prompt = self._build_json_prompt(question, context)

        @tenacity.retry(**self._retry_kwargs)
        def _call():
            return self.client.chat.completions.create(
                model=self.model,
                messages=[{"role": "user", "content": prompt}],
                max_tokens=1024,
                temperature=0.3
            )

        try:
            response = _call()
            raw_content = self._extract_content(response)
            parsed = self._parse_json_response(raw_content, context)
            if parsed:
                self.logger.debug("JSON mode parsing succeeded")
                return parsed
        except Exception as e:
            self.logger.debug(f"JSON mode failed, using fallback: {e}")

        return fallback_result

    def _build_json_prompt(self, question: str, context: List[Dict[str, Any]]) -> str:
        """Build prompt that requests JSON output."""
        context_texts = []
        for i, chunk in enumerate(context):
            context_texts.append(
                f"[Source {i+1}] ({chunk['source']}):\n{chunk['text']}"
            )
        context_section = "\n\n".join(context_texts)

        return f"""Based on the following context, answer the question. Return your response as a JSON object with this exact format:
{{
  "reasoning": "step-by-step explanation of how you derived the answer",
  "answer": "the final answer text",
  "cited_sources": [{{"source_idx": 1, "excerpt": "relevant text from source"}}, ...],
  "confidence": 0.0-1.0 (your confidence in the answer)
}}

Context:
{context_section}

Question: {question}

JSON Response:"""

    def _parse_json_response(
        self,
        raw_content: str,
        context: List[Dict[str, Any]]
    ) -> Optional[GenerationResult]:
        """Parse JSON response from model."""
        try:
            # Try to extract JSON from the content
            json_str = raw_content.strip()
            # Handle cases where model adds markdown code blocks
            if json_str.startswith("```"):
                lines = json_str.split("\n")
                json_str = "\n".join(lines[1:-1] if lines[-1].strip() == "```" else lines[1:])
                json_str = json_str.strip("`").strip()

            data = json.loads(json_str)
            cited_sources = []
            for cite in data.get("cited_sources", []):
                idx = cite.get("source_idx", 1) - 1
                if 0 <= idx < len(context):
                    cited_sources.append({
                        "source": context[idx]["source"],
                        "chunk_index": context[idx].get("chunk_index"),
                        "text": cite.get("excerpt", ""),
                        "relevance_score": context[idx].get("relevance_score")
                    })

            confidence = data.get("confidence")
            if confidence is not None:
                confidence = float(confidence)
                confidence = max(0.0, min(1.0, confidence))

            return GenerationResult(
                answer=data.get("answer", ""),
                reasoning=data.get("reasoning"),
                cited_sources=cited_sources,
                confidence=confidence,
                model_used=self.model
            )
        except (json.JSONDecodeError, KeyError, TypeError) as e:
            self.logger.debug(f"JSON parse failed: {e}, raw: {raw_content[:200]}")
            return None

    def _parse_structured_response(
        self,
        raw_content: str,
        context: List[Dict[str, Any]]
    ) -> GenerationResult:
        """Parse reasoning-style response and extract citations."""
        # Try JSON first
        json_result = self._parse_json_response(raw_content, context)
        if json_result:
            return json_result

        # Fallback: heuristic parsing of free-text reasoning
        reasoning, answer = self._split_reasoning_answer(raw_content)
        cited_sources = self._extract_citations(raw_content, context)
        confidence = self._estimate_confidence(raw_content, cited_sources, context)

        return GenerationResult(
            answer=answer or raw_content,
            reasoning=reasoning,
            cited_sources=cited_sources,
            confidence=confidence,
            model_used=self.model
        )

    def _split_reasoning_answer(self, text: str) -> tuple:
        """Split text into reasoning and final answer sections."""
        # Look for common separators between reasoning and answer
        separators = [
            r"(?i)final answer[:\s]+",
            r"(?i)answer[:\s]+",
            r"(?i)therefore[:\s]+",
            r"(?i)in conclusion[:\s]+",
            r"^---$",
            r"^\*\*Answer\*\*:",
        ]
        for sep in separators:
            match = re.search(sep, text, re.MULTILINE)
            if match:
                reasoning = text[:match.start()].strip()
                answer = text[match.start():].strip()
                # Remove the separator prefix from answer
                answer = re.sub(sep, "", answer, count=1, flags=re.MULTILINE).strip()
                return reasoning, answer
        return None, text

    def _extract_citations(
        self,
        text: str,
        context: List[Dict[str, Any]]
    ) -> List[Dict[str, Any]]:
        """Extract cited sources from answer text."""
        cited = []
        # Patterns like "Source 1", "[1]", "source 1", "according to source 1"
        patterns = [
            r"(?i)source\s+(\d+)",
            r"\[(\d+)\]",
            r"(?i)according to (?:source )?(\d+)",
            r"(?i)（来源\s*(\d+)）",
        ]
        cited_indices = set()
        for pat in patterns:
            matches = re.findall(pat, text)
            for m in matches:
                try:
                    idx = int(m) - 1
                    if 0 <= idx < len(context) and idx not in cited_indices:
                        cited_indices.add(idx)
                except ValueError:
                    pass

        for idx in cited_indices:
            cited.append({
                "source": context[idx]["source"],
                "chunk_index": context[idx].get("chunk_index"),
                "text": context[idx]["text"][:200],
                "relevance_score": context[idx].get("relevance_score")
            })
        return cited

    def _estimate_confidence(
        self,
        text: str,
        cited_sources: List[Dict],
        context: List[Dict[str, Any]]
    ) -> Optional[float]:
        """Estimate confidence based on answer characteristics."""
        if not cited_sources:
            return None

        text_lower = text.lower()
        # Deduct confidence for hedging language
        hedging = sum(1 for w in ["maybe", "perhaps", "might", "may", "possibly",
                                   "不确定", "可能", "也许", "大概"] if w in text_lower)
        hedge_factor = max(0, 1 - hedging * 0.15)

        # Boost confidence when sources are cited and relevant
        avg_relevance = sum(s.get("relevance_score", 0) for s in cited_sources) / len(cited_sources)
        relevance_factor = 0.5 + avg_relevance * 0.5

        # Combine: base 0.5 + factors
        confidence = 0.5 * hedge_factor + 0.5 * relevance_factor
        return round(max(0.0, min(1.0, confidence)), 2)

    def _generate_without_context(self, question: str) -> str:
        """Generate answer when no context is available.

        Args:
            question: User question.

        Returns:
            Explanatory message.
        """
        prompt = f"""Please answer the following question. If you don't have enough information, say so.

Question: {question}

Answer:"""

        @tenacity.retry(**self._retry_kwargs)
        def _call():
            return self.client.chat.completions.create(
                model=self.model,
                messages=[{"role": "user", "content": prompt}],
                max_tokens=1024,
                temperature=0.7
            )

        response = _call()
        return self._extract_content(response)
