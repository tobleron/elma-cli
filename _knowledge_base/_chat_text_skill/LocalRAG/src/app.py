"""Streamlit application for Local RAG."""

import streamlit as st
from pathlib import Path
import yaml

import observability
from observability import get_logger

# Initialize observability at app startup (logs to logs/app.log)
observability.configure_logging(log_level="INFO", log_file="logs/app.log")
logger = get_logger(__name__)

# Start Prometheus metrics server on port 9090 (daemon thread)
METRICS_PORT = 9090
try:
    metrics_thread = observability.start_metrics_server(port=METRICS_PORT)
    logger.info("Metrics server started", port=METRICS_PORT)
except Exception as e:
    logger.warning("Failed to start metrics server", error=str(e))

from pipeline import RAGPipeline
from llm_api import LLMAPIClient
from embedder_api import EmbedderAPIClient
from chunkers import create_chunker, list_chunkers
from history_manager import HistoryManager

# Import chunker modules to trigger registration
import chunkers.fixed_size_chunker  # noqa: F401
import chunkers.recursive_chunker  # noqa: F401
import chunkers.structure_chunker  # noqa: F401
import chunkers.semantic_chunker  # noqa: F401
import chunkers.llm_chunker  # noqa: F401


st.set_page_config(
    page_title="Local RAG",
    page_icon="📚",
    layout="wide"
)

API_CONFIG_FILE = "config/api_settings.yaml"


def load_api_config():
    """Load API configuration from file."""
    if Path(API_CONFIG_FILE).exists():
        with open(API_CONFIG_FILE, "r") as f:
            return yaml.safe_load(f)
    return None


def save_api_config(llm_config: dict, embed_config: dict, rerank_config: dict):
    """Save API configuration to file."""
    Path(API_CONFIG_FILE).parent.mkdir(parents=True, exist_ok=True)
    config = {
        "llm": llm_config,
        "embedding": embed_config,
        "rerank": rerank_config
    }
    with open(API_CONFIG_FILE, "w") as f:
        yaml.dump(config, f)


def init_session_state():
    """Initialize Streamlit session state."""
    if "pipeline" not in st.session_state:
        st.session_state.pipeline = None
    if "llm_client" not in st.session_state:
        st.session_state.llm_client = None
    if "embedder_client" not in st.session_state:
        st.session_state.embedder_client = None
    if "messages" not in st.session_state:
        st.session_state.messages = []
    if "selected_docs" not in st.session_state:
        st.session_state.selected_docs = []  # Empty list means all documents
    if "pipeline_initialized" not in st.session_state:
        st.session_state.pipeline_initialized = False
    # New: chunking state
    if "chunking_strategy" not in st.session_state:
        st.session_state.chunking_strategy = "fixed"
    if "chunking_params" not in st.session_state:
        st.session_state.chunking_params = {"chunk_size": 512, "overlap": 50}
    if "preview_text" not in st.session_state:
        st.session_state.preview_text = ""
    if "preview_results" not in st.session_state:
        st.session_state.preview_results = None
    # Load saved API config
    if "saved_api_config" not in st.session_state:
        st.session_state.saved_api_config = load_api_config()
    # History manager and conversation state
    if "history_manager" not in st.session_state:
        st.session_state.history_manager = HistoryManager()
    if "conversation_id" not in st.session_state:
        st.session_state.conversation_id = None
    if "current_conv_loaded" not in st.session_state:
        st.session_state.current_conv_loaded = False  # tracks if messages are from a loaded conversation


def auto_apply_config():
    """Auto-apply saved configuration if available."""
    saved = st.session_state.get("saved_api_config")
    if saved and not st.session_state.pipeline_initialized:
        try:
            llm_config = saved.get("llm", {})
            embed_config = saved.get("embedding", {})
            rerank_config = saved.get("rerank", {})

            st.session_state.llm_client = LLMAPIClient(
                api_base=llm_config.get("api_base", "http://127.0.0.1:1234/v1"),
                api_key=llm_config.get("api_key", "not-needed"),
                model=llm_config.get("model", "llama3.2")
            )
            st.session_state.embedder_client = EmbedderAPIClient(
                api_base=embed_config.get("api_base", "http://127.0.0.1:1234/v1"),
                api_key=embed_config.get("api_key", "not-needed"),
                model=embed_config.get("model", "embedding-model")
            )
            st.session_state.pipeline = RAGPipeline()
            # Apply rerank settings from api_settings.yaml
            rer = st.session_state.pipeline.config["retrieval"]
            if rerank_config.get("api_base"):
                rer["rerank_api_base"] = rerank_config.get("api_base", "")
                rer["rerank_api_key"] = rerank_config.get("api_key", "not-needed")
            if rerank_config.get("model"):
                rer["rerank_model"] = rerank_config.get("model", "")
            st.session_state.pipeline_initialized = True
            logger.info("Auto-applied saved config successfully")
            return True
        except Exception as e:
            logger.error("Failed to auto-apply config", error=str(e))
            return False
    return False


def create_api_clients(llm_config: dict, embedder_config: dict):
    """Create API clients based on configuration."""
    llm_client = LLMAPIClient(
        api_base=llm_config["api_base"],
        api_key=llm_config.get("api_key", "not-needed"),
        model=llm_config["model"]
    )
    embedder_client = EmbedderAPIClient(
        api_base=embedder_config["api_base"],
        api_key=embedder_config.get("api_key", "not-needed"),
        model=embedder_config["model"]
    )
    return llm_client, embedder_client


def main():
    """Main Streamlit application."""
    init_session_state()

    # Auto-apply saved configuration if available
    if auto_apply_config():
        pass  # Configuration was auto-applied

    st.title("📚 Local RAG System")

    # Feature tabs
    tab_config, tab_chunking, tab_docs, tab_rag, tab_observe = st.tabs(
        ["⚙️ Configuration", "🔪 Chunking", "📁 Documents", "💬 RAG", "📊 Observability"]
    )

    # Get saved config for default values
    saved = st.session_state.get("saved_api_config", {})
    default_llm = saved.get("llm", {}) if saved else {}
    default_embed = saved.get("embedding", {}) if saved else {}
    default_rerank = saved.get("rerank", {}) if saved else {}

    with tab_config:
        st.header("⚙️ API Configuration")

        col1, col2 = st.columns(2)

        with col1:
            st.subheader("🔌 LLM API Settings")
            llm_api_base = st.text_input(
                "API Base URL",
                value=default_llm.get("api_base", "http://127.0.0.1:1234/v1"),
                help="OpenAI-compatible API base URL (e.g., LM Studio, Ollama)"
            )
            llm_api_key = st.text_input(
                "API Key",
                value=default_llm.get("api_key", "not-needed"),
                type="password",
                help="API key (often not needed for local servers)"
            )
            llm_model = st.text_input(
                "LLM Model Name",
                value=default_llm.get("model", "llama3.2"),
                help="Model name in your API server"
            )

        with col2:
            st.subheader("📊 Embedding API Settings")
            embed_api_base = st.text_input(
                "Embedding API Base URL",
                value=default_embed.get("api_base", "http://127.0.0.1:1234/v1"),
                help="OpenAI-compatible embedding API base URL"
            )
            embed_api_key = st.text_input(
                "Embedding API Key",
                value=default_embed.get("api_key", "not-needed"),
                type="password"
            )
            embed_model = st.text_input(
                "Embedding Model Name",
                value=default_embed.get("model", "embedding-model"),
                help="Embedding model name in your API server"
            )

        # Rerank model configuration
        st.subheader("🎯 Reranking API (Optional)")
        col_r1, col_r2 = st.columns(2)
        with col_r1:
            rerank_api_base = st.text_input(
                "Rerank API Base URL",
                value=default_rerank.get("api_base", ""),
                help="Base URL for reranking API (e.g. http://localhost:8000/v1). If provided, uses API mode."
            )
        with col_r2:
            rerank_api_key = st.text_input(
                "Rerank API Key",
                value=default_rerank.get("api_key", "not-needed"),
                type="password",
                help="API key for reranking service (optional for local servers)"
            )
        rerank_model = st.text_input(
            "Cross-Encoder Model Path",
            value=default_rerank.get("model", ""),
            help="Local path to a downloaded Cross-Encoder model, or HuggingFace model ID. Leave empty to use API only or to disable."
        )

        if st.button("🔄 Apply Configuration", use_container_width=True):
            with st.spinner("Connecting to API..."):
                try:
                    llm_config = {
                        "api_base": llm_api_base,
                        "api_key": llm_api_key,
                        "model": llm_model
                    }
                    embed_config = {
                        "api_base": embed_api_base,
                        "api_key": embed_api_key,
                        "model": embed_model
                    }
                    rerank_config = {
                        "api_base": rerank_api_base,
                        "api_key": rerank_api_key,
                        "model": rerank_model
                    }
                    # Save config to file
                    save_api_config(llm_config, embed_config, rerank_config)
                    st.session_state.saved_api_config = {"llm": llm_config, "embedding": embed_config, "rerank": rerank_config}

                    st.session_state.llm_client, st.session_state.embedder_client = create_api_clients(llm_config, embed_config)
                    st.session_state.pipeline = RAGPipeline()
                    # Apply rerank settings from config
                    rer = st.session_state.pipeline.config["retrieval"]
                    if rerank_api_base:
                        rer["rerank_api_base"] = rerank_api_base
                        rer["rerank_api_key"] = rerank_api_key
                    if rerank_model:
                        rer["rerank_model"] = rerank_model
                        st.session_state.pipeline.config["retrieval"]["rerank_model"] = rerank_model
                    st.session_state.pipeline_initialized = True
                    logger.info("Configuration applied successfully")
                    st.success("✅ Connected to API!")
                except Exception as e:
                    observability.log_error_alert(logger, e, "app",
                                context={"operation": "apply_config"})
                    st.error(f"❌ Connection failed: {e}")

        # Retrieval Enhancement Settings
        if st.session_state.pipeline_initialized:
            with st.expander("🔍 Retrieval Enhancements", expanded=False):
                st.markdown("Configure hybrid search, reranking, and result diversity.")

                retrieval_cfg = st.session_state.pipeline.config.get("retrieval", {})

                col_a, col_b = st.columns(2)
                with col_a:
                    hybrid_search = st.toggle(
                        "Hybrid Search (Dense + BM25)",
                        value=retrieval_cfg.get("hybrid_search", False),
                        help="Combine vector similarity with BM25 keyword search using Reciprocal Rank Fusion"
                    )
                    rerank = st.toggle(
                        "Reranking (Cross-Encoder)",
                        value=retrieval_cfg.get("rerank", False),
                        help="Use a cross-encoder model to reorder retrieval results for higher relevance"
                    )
                    mmr_enabled = st.toggle(
                        "MMR Diversity",
                        value=retrieval_cfg.get("mmr_enabled", False),
                        help="Maximal Marginal Relevance — diversifies results to avoid redundant content"
                    )

                with col_b:
                    top_k = st.slider(
                        "Top-K Results", 1, 20,
                        value=retrieval_cfg.get("top_k", 5),
                        help="Final number of chunks returned to LLM"
                    )
                    dense_top_k = st.slider(
                        "Dense Retrieval Count", 5, 50,
                        value=retrieval_cfg.get("dense_top_k", 20),
                        help="Number of vector search results before reranking/fusion"
                    )
                    sparse_top_k = st.slider(
                        "BM25 Retrieval Count", 5, 50,
                        value=retrieval_cfg.get("sparse_top_k", 20),
                        help="Number of BM25 results before fusion"
                    )

                if hybrid_search or mmr_enabled:
                    col_c, col_d = st.columns(2)
                    with col_c:
                        mmr_lambda = st.slider(
                            "MMR Lambda (Diversity Weight)", 0.0, 1.0,
                            value=float(retrieval_cfg.get("mmr_lambda", 0.3)),
                            step=0.05,
                            help="0.0 = pure relevance, 1.0 = pure diversity"
                        )
                    with col_d:
                        rerank_top_k = st.slider(
                            "Rerank Candidate Count", 5, 50,
                            value=retrieval_cfg.get("rerank_top_k", 20),
                            help="Number of candidates passed to reranker"
                        )

                if st.button("💾 Save Retrieval Settings"):
                    # Update config in memory (Note: full persistence would need config rewrite)
                    st.session_state.pipeline.config["retrieval"]["hybrid_search"] = hybrid_search
                    st.session_state.pipeline.config["retrieval"]["rerank"] = rerank
                    st.session_state.pipeline.config["retrieval"]["mmr_enabled"] = mmr_enabled
                    st.session_state.pipeline.config["retrieval"]["top_k"] = top_k
                    st.session_state.pipeline.config["retrieval"]["dense_top_k"] = dense_top_k
                    st.session_state.pipeline.config["retrieval"]["sparse_top_k"] = sparse_top_k
                    if hybrid_search or mmr_enabled:
                        st.session_state.pipeline.config["retrieval"]["mmr_lambda"] = mmr_lambda
                        st.session_state.pipeline.config["retrieval"]["rerank_top_k"] = rerank_top_k
                    st.success("✅ Retrieval settings updated (current session)")

        if st.session_state.pipeline_initialized:
            st.info("✅ Configuration applied")

    with tab_chunking:
        if not st.session_state.pipeline_initialized:
            st.warning("⚠️ Please configure API in the Configuration tab first")
        else:
            # Section 1: Strategy & Parameters
            with st.expander("⚙️ Chunking Strategy & Parameters", expanded=True):
                col1, col2 = st.columns([1, 1])

                with col1:
                    strategy = st.selectbox(
                        "Strategy:",
                        options=["fixed", "recursive", "structure", "semantic", "llm"],
                        index=["fixed", "recursive", "structure", "semantic", "llm"].index(st.session_state.chunking_strategy) if st.session_state.chunking_strategy in ["fixed", "recursive", "structure", "semantic", "llm"] else 0,
                        format_func=lambda x: {
                            "fixed": "Fixed Size",
                            "recursive": "Recursive",
                            "structure": "Document Structure",
                            "semantic": "Semantic",
                            "llm": "LLM-based"
                        }[x]
                    )
                    st.session_state.chunking_strategy = strategy

                    chunk_size = st.number_input(
                        "Chunk Size (chars)",
                        min_value=50,
                        max_value=2000,
                        value=st.session_state.chunking_params.get("chunk_size", 512)
                    )

                with col2:
                    overlap = st.number_input(
                        "Overlap (chars)",
                        min_value=0,
                        max_value=200,
                        value=st.session_state.chunking_params.get("overlap", 50)
                    )

                    # Strategy-specific params
                    if strategy == "semantic":
                        similarity_threshold = st.slider(
                            "Similarity Threshold",
                            min_value=0.0,
                            max_value=1.0,
                            value=0.7,
                            step=0.05
                        )
                        min_chunk_size = st.number_input(
                            "Min Chunk Size",
                            min_value=10,
                            max_value=500,
                            value=100
                        )
                        params = {"similarity_threshold": similarity_threshold, "min_chunk_size": min_chunk_size}
                    elif strategy == "recursive":
                        separators = st.multiselect(
                            "Separators",
                            options=["\n\n", "\n", ". ", " "],
                            default=["\n\n", "\n", ". ", " "],
                            format_func=lambda x: {"\n\n": "¶¶", "\n": "¶", ". ": ". ", " ": "space"}[x]
                        )
                        params = {"separators": separators}
                    elif strategy == "structure":
                        split_on = st.selectbox(
                            "Split On",
                            options=["paragraphs", "headings", "lists", "code_blocks"],
                            format_func=lambda x: x.capitalize()
                        )
                        params = {"split_on": split_on}
                    elif strategy == "llm":
                        max_llm_chunk_size = st.number_input(
                            "Max LLM Chunk Size (chars)",
                            min_value=500,
                            max_value=5000,
                            value=st.session_state.chunking_params.get("max_llm_chunk_size", 2000),
                            step=100,
                            help="Maximum text size to send to LLM at once"
                        )
                        fallback = st.checkbox(
                            "Fallback to sentence splitting",
                            value=True,
                            help="If LLM fails or unavailable, fall back to sentence splitting"
                        )
                        params = {"max_llm_chunk_size": max_llm_chunk_size, "fallback_to_sentence": fallback}
                        # Pass LLM client if available
                        if st.session_state.get("llm_client"):
                            params["llm_client"] = st.session_state.llm_client
                    else:
                        params = {}

                    all_params = {"chunk_size": chunk_size, "overlap": overlap, **params}
                    st.session_state.chunking_params = all_params

            # Section 2: File Upload
            st.subheader("📁 Upload Document")
            uploaded_file = st.file_uploader(
                "Choose a file",
                type=["txt", "md", "pdf"],
                help="Upload TXT, Markdown, or PDF files"
            )

            if uploaded_file is not None:
                with st.spinner("📄 Reading document..."):
                    try:
                        from loader import DocumentLoader
                        loader = DocumentLoader()
                        temp_path = Path("data/uploads") / uploaded_file.name
                        temp_path.parent.mkdir(parents=True, exist_ok=True)

                        with open(temp_path, "wb") as f:
                            f.write(uploaded_file.getbuffer())

                        doc_data = loader.load_with_metadata(temp_path)
                        st.session_state.preview_text = doc_data["content"]
                        st.success(f"✅ Loaded '{uploaded_file.name}' ({len(doc_data['content'])} chars)")
                    except Exception as e:
                        st.error(f"❌ Error reading file: {e}")

            # Section 3: Text Input
            st.subheader("📝 Text to Chunk")
            preview_text = st.text_area(
                "Enter or paste text here:",
                value=st.session_state.preview_text,
                height=150,
                placeholder="Type or paste text to preview chunking..."
            )
            st.session_state.preview_text = preview_text

            # Section 4: Actions
            col_btn1, col_btn2 = st.columns(2)
            with col_btn1:
                if st.button("🔍 Preview", use_container_width=True, type="primary"):
                    if preview_text:
                        try:
                            chunker = create_chunker(strategy, **all_params)
                            result = chunker.preview(preview_text)
                            st.session_state.preview_results = result
                        except Exception as e:
                            st.error(f"Preview failed: {e}")
                    else:
                        st.warning("Please enter some text first")

            with col_btn2:
                if st.button("📚 Index Text", use_container_width=True):
                    if preview_text:
                        try:
                            import tempfile
                            import os

                            # Write text as UTF-8 encoded bytes
                            with tempfile.NamedTemporaryFile(mode='wb', suffix='.txt', delete=False) as f:
                                f.write(preview_text.encode('utf-8'))
                                temp_path = f.name

                            try:
                                from chunker import TextChunker as LegacyChunker
                                legacy_chunker = LegacyChunker(
                                    chunk_size=chunk_size,
                                    chunk_overlap=overlap,
                                    strategy=strategy
                                )
                                old_chunker = st.session_state.pipeline.chunker
                                st.session_state.pipeline.chunker = legacy_chunker

                                num_chunks = st.session_state.pipeline.ingest_document(
                                    temp_path,
                                    embedder=st.session_state.embedder_client
                                )
                                st.success(f"✅ Indexed {num_chunks} chunks")
                            finally:
                                st.session_state.pipeline.chunker = old_chunker
                                os.unlink(temp_path)
                        except Exception as e:
                            st.error(f"Indexing failed: {e}")
                    else:
                        st.warning("Please enter some text first")

            # Section 5: Results
            if st.session_state.preview_results:
                result = st.session_state.preview_results
                stats = result["stats"]

                st.subheader("📊 Preview Results")
                col_s1, col_s2, col_s3, col_s4 = st.columns(4)
                with col_s1:
                    st.metric("Total Chunks", stats["total_chunks"])
                with col_s2:
                    st.metric("Avg Size", stats["avg_size"])
                with col_s3:
                    st.metric("Min Size", stats["min_size"])
                with col_s4:
                    st.metric("Max Size", stats["max_size"])

                st.subheader("📋 Chunks")
                for chunk in result["chunks"]:
                    with st.expander(f"[{chunk['index']}] {chunk['text'][:60]}... ({chunk['size']} chars)"):
                        st.text(chunk["text"])

    with tab_docs:
        st.header("📁 Document Management")

        if not st.session_state.pipeline_initialized:
            st.warning("⚠️ Please configure API in the Configuration tab first")
        else:
            # Refresh button
            if st.button("🔄 Refresh"):
                st.rerun()

            try:
                docs = st.session_state.pipeline.vectorstore.get_indexed_documents()
                if not docs:
                    st.info("No documents indexed yet. Go to the Chunking tab to add documents.")
                else:
                    total_chunks = sum(d["chunk_count"] for d in docs)
                    col_stat1, col_stat2 = st.columns(2)
                    with col_stat1:
                        st.metric("Total Documents", len(docs))
                    with col_stat2:
                        st.metric("Total Chunks", total_chunks)

                    st.divider()

                    # Clear all button
                    if st.button("🗑️ Clear All Documents", type="secondary"):
                        st.session_state.pipeline.vectorstore.clear_all()
                        st.success("All documents cleared.")
                        st.rerun()

                    st.divider()
                    st.subheader("📋 Indexed Documents")

                    for doc in docs:
                        col_name, col_chunks, col_action = st.columns([3, 1, 1])
                        with col_name:
                            st.text(doc["source_file"])
                        with col_chunks:
                            st.caption(f"{doc['chunk_count']} chunks")
                        with col_action:
                            if st.button("Delete", key=f"del_{doc['source_file']}"):
                                try:
                                    count = st.session_state.pipeline.vectorstore.delete_document(doc["source_file"])
                                    st.success(f"Deleted {count} chunks.")
                                    st.rerun()
                                except Exception as e:
                                    st.error(f"Error: {e}")
                        st.divider()
            except Exception as e:
                st.error(f"Error loading documents: {e}")

    with tab_rag:
        st.header("💬 Ask Questions")

        if not st.session_state.pipeline_initialized:
            st.info("👈 Please configure the system using the sidebar first")
        else:
            # ── Sidebar: Chat History ──────────────────────────────────
            hm = st.session_state.history_manager

            with st.sidebar:
                st.subheader("💬 Chat History")

                if st.button("➕ New Chat", use_container_width=True):
                    # Clear current conversation in UI
                    st.session_state.messages = []
                    st.session_state.conversation_id = None
                    st.session_state.current_conv_loaded = False
                    st.rerun()

                st.divider()

                # Feedback stats
                stats = hm.get_feedback_stats()
                col_f1, col_f2 = st.columns(2)
                with col_f1:
                    st.markdown(f"👍 {stats['thumbs_up']}")
                with col_f2:
                    st.markdown(f"👎 {stats['thumbs_down']}")

                st.divider()

                # Conversation list
                conversations = hm.list_conversations()
                if not conversations:
                    st.caption("No conversations yet.")
                else:
                    for conv in conversations:
                        col_title, col_del = st.columns([4, 1])
                        with col_title:
                            is_active = st.session_state.conversation_id == conv["id"]
                            btn_label = f"**{conv['title']}**" if is_active else conv["title"]
                            if st.button(
                                btn_label,
                                key=f"conv_{conv['id']}",
                                help=f"{conv['message_count']} messages"
                            ):
                                # Load this conversation
                                msgs = hm.get_conversation(conv["id"])
                                st.session_state.messages = msgs
                                st.session_state.conversation_id = conv["id"]
                                st.session_state.current_conv_loaded = True
                                st.rerun()
                        with col_del:
                            if st.button("🗑️", key=f"del_conv_{conv['id']}"):
                                hm.delete_conversation(conv["id"])
                                if st.session_state.conversation_id == conv["id"]:
                                    st.session_state.messages = []
                                    st.session_state.conversation_id = None
                                    st.session_state.current_conv_loaded = False
                                st.rerun()

            # ── Main area: Document filter ─────────────────────────────
            try:
                docs = st.session_state.pipeline.vectorstore.get_indexed_documents()
                if docs:
                    doc_names = [d["source_file"] for d in docs]
                    all_docs_option = ["📚 All Documents"]
                    selected = st.multiselect(
                        "Filter by document:",
                        options=all_docs_option + doc_names,
                        default=st.session_state.selected_docs if st.session_state.selected_docs else [all_docs_option[0]],
                        format_func=lambda x: x
                    )
                    if all_docs_option[0] in selected or not selected:
                        st.session_state.selected_docs = []
                    else:
                        st.session_state.selected_docs = selected
            except Exception:
                pass

            # ── Display messages ────────────────────────────────────────
            for msg_idx, message in enumerate(st.session_state.messages):
                with st.chat_message(message["role"]):
                    st.markdown(message["content"])

                    # Feedback buttons for assistant messages
                    if message["role"] == "assistant":
                        sources = message.get("sources", [])
                        # Find the corresponding question for this answer
                        question = ""
                        for i in range(msg_idx - 1, -1, -1):
                            if st.session_state.messages[i]["role"] == "user":
                                question = st.session_state.messages[i]["content"]
                                break

                        col_fb, col_spacer = st.columns([1, 5])
                        with col_fb:
                            c1, c2 = st.columns(2)
                            with c1:
                                if st.button("👍", key=f"up_{msg_idx}"):
                                    if question:
                                        hm.save_feedback(question, message["content"], "thumbs_up", sources)
                                    st.rerun()
                            with c2:
                                if st.button("👎", key=f"down_{msg_idx}"):
                                    if question:
                                        hm.save_feedback(question, message["content"], "thumbs_down", sources)
                                    st.rerun()

            # ── Message input ───────────────────────────────────────────
            if question := st.chat_input("Ask a question about your documents..."):
                # If switching from a loaded conversation to new messages, create new conversation
                if st.session_state.current_conv_loaded:
                    # User is continuing from loaded conversation - append to it
                    pass
                elif st.session_state.conversation_id is None:
                    # Brand new conversation
                    conv_id = hm.create_conversation()
                    st.session_state.conversation_id = conv_id
                    st.session_state.current_conv_loaded = False

                conv_id = st.session_state.conversation_id

                # Save user message
                st.session_state.messages.append({"role": "user", "content": question})
                hm.append_message(conv_id, "user", question)

                with st.chat_message("user"):
                    st.markdown(question)

                with st.chat_message("assistant"):
                    with st.spinner("🔍 Searching and generating..."):
                        try:
                            result = st.session_state.pipeline.query(
                                question,
                                filter_sources=st.session_state.selected_docs if st.session_state.selected_docs else None,
                                llm_client=st.session_state.llm_client,
                                embedder_client=st.session_state.embedder_client
                            )

                            # Display reasoning chain if available
                            if result.get("reasoning"):
                                with st.expander("🧠 Reasoning Chain"):
                                    st.markdown(result["reasoning"])

                            # Display confidence indicator
                            confidence = result.get("confidence")
                            if confidence is not None:
                                conf_color = "green" if confidence >= 0.7 else "orange" if confidence >= 0.4 else "red"
                                st.markdown(
                                    f"**Confidence:** :{conf_color}[{confidence:.0%}]"
                                )

                            st.markdown(result["answer"])

                            # Show cited sources with excerpts
                            cited = result.get("cited_sources", [])
                            if cited:
                                with st.expander(f"📚 Cited Sources ({len(cited)})"):
                                    for i, src in enumerate(cited):
                                        st.markdown(f"**Source {i+1}** ({src['source']})")
                                        st.text(src.get("text", "")[:300] +
                                               ("..." if len(src.get("text", "")) > 300 else ""))
                                        st.markdown("---")

                            # Full sources in separate expander
                            sources = result.get("sources", [])
                            if sources:
                                with st.expander("📄 All Retrieved Sources"):
                                    for i, source in enumerate(sources):
                                        st.markdown(f"**Source {i+1}** ({source['source']}):")
                                        st.text(source["text"][:300] + "..." if len(source["text"]) > 300 else source["text"])
                                        st.markdown("---")

                            # Save to history
                            assistant_msg = {"role": "assistant", "content": result["answer"]}
                            if sources:
                                assistant_msg["sources"] = sources
                            st.session_state.messages.append(assistant_msg)
                            hm.append_message(conv_id, "assistant", result["answer"], sources)
                        except Exception as e:
                            observability.log_error_alert(logger, e, "app",
                                        context={"operation": "rag_query", "question_length": len(question)})
                            st.error(f"❌ Error: {e}")

    with tab_observe:
        st.header("📊 Observability Dashboard")

        col_logs, col_metrics = st.columns(2)

        with col_logs:
            st.subheader("📋 Recent Logs")
            if st.button("🔄 Refresh Logs"):
                st.rerun()

            log_content = observability.read_recent_logs("logs/app.log", lines=50)
            st.text_area(
                "Log output (last 50 lines)",
                value=log_content,
                height=400,
                disabled=True,
                label_visibility="collapsed"
            )

        with col_metrics:
            st.subheader("📈 Metrics & Health")

            # Prometheus metrics info
            st.markdown(f"""
**Prometheus Metrics:** `http://localhost:{METRICS_PORT}/metrics`

Scrape this endpoint with Prometheus to collect:
- `rag_query_duration_seconds` — RAG query latency by phase
- `rag_query_total` — total RAG queries by status
- `document_ingest_duration_seconds` — ingestion latency
- `api_call_duration_seconds` — LLM/embedder API latency
- `api_errors_total` — API errors by type
- `vectorstore_operation_duration_seconds` — vector DB operation latency
            """)

            # Quick stats from in-memory metrics (approximate)
            st.divider()
            st.subheader("🔍 Live Metrics Preview")

            # Check if vectorstore has data
            if st.session_state.pipeline_initialized:
                try:
                    docs = st.session_state.pipeline.vectorstore.get_indexed_documents()
                    total_chunks = sum(d["chunk_count"] for d in docs)
                    col_m1, col_m2 = st.columns(2)
                    with col_m1:
                        st.metric("Indexed Documents", len(docs))
                    with col_m2:
                        st.metric("Total Chunks", total_chunks)
                except Exception:
                    st.info("Vector store not available")
            else:
                st.info("Pipeline not initialized — configure API first")

            # Feedback stats if history manager exists
            if "history_manager" in st.session_state:
                stats = st.session_state.history_manager.get_feedback_stats()
                col_f1, col_f2 = st.columns(2)
                with col_f1:
                    st.metric("👍 Positive Feedback", stats["thumbs_up"])
                with col_f2:
                    st.metric("👎 Negative Feedback", stats["thumbs_down"])

            st.divider()
            st.subheader("📁 Log Files")
            st.markdown("""
**Log location:** `logs/app.log`

Log files are rotated (max 10 MB per file, 5 backups kept):
- `logs/app.log` — current log
- `logs/app.log.1`, `logs/app.log.2`, ... — rotated backups
            """)


if __name__ == "__main__":
    main()