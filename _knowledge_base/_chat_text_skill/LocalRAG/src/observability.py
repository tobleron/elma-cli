"""Observability module for LocalRAG - structured logging, metrics, and tracing."""

import structlog
import logging
import logging.handlers
import sys
import time
import uuid
from functools import wraps
from pathlib import Path
from typing import Dict, Any, Optional, Callable
from threading import Thread
from wsgiref.simple_server import make_server, WSGIServer
from socketserver import ThreadingMixIn
from prometheus_client import Counter, Histogram, Gauge, make_wsgi_app
from contextvars import ContextVar


# ============================================================================
# Context Variables
# ============================================================================

_request_context: ContextVar[Dict[str, Any]] = ContextVar("request_context", default={})


def get_request_id() -> Optional[str]:
    """Get current request ID from context."""
    return _request_context.get().get("request_id")


def get_current_context() -> Dict[str, Any]:
    """Get full current context (read-only copy)."""
    return _request_context.get().copy()


# ============================================================================
# Metrics Definitions
# ============================================================================

# RAG Query Metrics
rag_query_latency = Histogram(
    "rag_query_duration_seconds",
    "RAG query total latency",
    ["phase"],
    buckets=[0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
)
rag_query_total = Counter(
    "rag_query_total",
    "Total RAG queries",
    ["status"]
)
active_rag_requests = Gauge(
    "active_rag_requests",
    "Number of currently active RAG requests"
)

# Document Ingestion Metrics
ingest_latency = Histogram(
    "document_ingest_duration_seconds",
    "Document ingestion latency",
    ["phase"],
    buckets=[0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]
)
ingest_total = Counter(
    "document_ingest_total",
    "Total document ingestions",
    ["status"]
)
ingest_chunks = Histogram(
    "document_ingest_chunks",
    "Number of chunks per document",
    ["phase"],
    buckets=[1, 5, 10, 25, 50, 100, 250]
)

# API Call Metrics
api_latency = Histogram(
    "api_call_duration_seconds",
    "External API call latency",
    ["client", "operation"],
    buckets=[0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
)
api_errors = Counter(
    "api_errors_total",
    "External API errors",
    ["client", "error_type"]
)

# Vector Store Metrics
vectorstore_operations = Histogram(
    "vectorstore_operation_duration_seconds",
    "Vector store operation latency",
    ["operation"],
    buckets=[0.01, 0.05, 0.1, 0.25, 0.5, 1.0]
)
vectorstore_errors = Counter(
    "vectorstore_errors_total",
    "Vector store errors",
    ["operation", "error_type"]
)

# Error Metrics
errors_by_type = Counter(
    "errors_total",
    "Total errors by type and module",
    ["module", "error_type"]
)


# ============================================================================
# Error Categorization
# ============================================================================

class ErrorType:
    """Error type constants for categorization."""
    API_ERROR = "API_ERROR"
    VALIDATION_ERROR = "VALIDATION_ERROR"
    STORAGE_ERROR = "STORAGE_ERROR"
    CHUNKING_ERROR = "CHUNKING_ERROR"
    LOADER_ERROR = "LOADER_ERROR"
    UNKNOWN_ERROR = "UNKNOWN_ERROR"


# ============================================================================
# Logger Configuration
# ============================================================================

def configure_logging(log_level: str = "INFO", log_file: str = "logs/app.log") -> structlog.BoundLogger:
    """Configure structlog with JSON output and context injection.

    Args:
        log_level: Minimum log level ("DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL")
        log_file: Path to the log file. If empty, only stdout is used.

    Returns:
        Configured structlog logger instance
    """
    structlog.configure(
        processors=[
            structlog.contextvars.merge_contextvars,
            structlog.stdlib.add_log_level,
            structlog.stdlib.add_logger_name,
            structlog.processors.TimeStamper(fmt="iso"),
            structlog.processors.StackInfoRenderer(),
            structlog.processors.format_exc_info,
            structlog.processors.JSONRenderer()
        ],
        wrapper_class=structlog.stdlib.BoundLogger,
        context_class=dict,
        logger_factory=structlog.stdlib.LoggerFactory(),
        cache_logger_on_first_use=True,
    )

    # Stdout handler (always)
    stdout_handler = logging.StreamHandler(sys.stdout)
    stdout_handler.setFormatter(logging.Formatter("%(message)s"))

    # File handler with rotation (if log_file specified)
    root_logger = logging.getLogger()
    root_logger.setLevel(getattr(logging, log_level.upper(), logging.INFO))
    root_logger.addHandler(stdout_handler)

    if log_file:
        log_path = Path(log_file)
        log_path.parent.mkdir(parents=True, exist_ok=True)
        file_handler = logging.handlers.RotatingFileHandler(
            log_file,
            maxBytes=10 * 1024 * 1024,  # 10 MB per file
            backupCount=5,                # Keep 5 backup files
        )
        file_handler.setFormatter(logging.Formatter("%(message)s"))
        root_logger.addHandler(file_handler)

    return structlog.get_logger("observability")


# ============================================================================
# Logger Access
# ============================================================================

def get_logger(name: str) -> structlog.BoundLogger:
    """Get a structured logger for a module.

    Args:
        name: Logger name (typically __name__)

    Returns:
        Structlog BoundLogger with context awareness
    """
    return structlog.get_logger(name)


# ============================================================================
# Request Context Management
# ============================================================================

class request_context:
    """Context manager for request-scoped observability data.

    Usage:
        with request_context(request_id="abc", operation="query"):
            log.info("Starting query")
    """

    def __init__(self, request_id: Optional[str] = None, **kwargs):
        self.request_id = request_id or str(uuid.uuid4())[:8]
        self._token = None
        self._extra = kwargs

    def __enter__(self) -> Dict[str, Any]:
        current = _request_context.get()
        self._token = _request_context.set({
            "request_id": self.request_id,
            **current,
            **self._extra
        })
        return get_current_context()

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self._token is not None:
            _request_context.reset(self._token)
        return False


# ============================================================================
# Metrics Helpers
# ============================================================================

def observe_latency(histogram: Histogram, labels: Dict[str, str], duration: float):
    """Observe a latency value in a histogram."""
    histogram.labels(**labels).observe(duration)


def increment_counter(counter: Counter, labels: Dict[str, str], value: int = 1):
    """Increment a counter."""
    counter.labels(**labels).inc(value)


# ============================================================================
# Error Handling Helpers
# ============================================================================

def get_error_type(exception: Exception) -> str:
    """Categorize an exception into an error type."""
    exc_type = type(exception).__name__.lower()

    if "api" in exc_type or "timeout" in exc_type or "connection" in exc_type:
        return ErrorType.API_ERROR
    elif "validation" in exc_type or "value" in exc_type or "type" in exc_type:
        return ErrorType.VALIDATION_ERROR
    elif "storage" in exc_type or "chroma" in exc_type or "persist" in exc_type:
        return ErrorType.STORAGE_ERROR
    elif "chunk" in exc_type or "split" in exc_type:
        return ErrorType.CHUNKING_ERROR
    elif "load" in exc_type or "read" in exc_type or "pdf" in exc_type:
        return ErrorType.LOADER_ERROR
    else:
        return ErrorType.UNKNOWN_ERROR


def log_error_alert(
    logger: structlog.BoundLogger,
    error: Exception,
    module: str,
    context: Optional[Dict[str, Any]] = None,
    severity: str = "ERROR"
):
    """Log an error with full context and categorization."""
    error_type = get_error_type(error)
    error_id = str(uuid.uuid4())[:8]

    extra = {
        "error_id": error_id,
        "error_type": error_type,
        "module": module,
        "error_class": type(error).__name__,
        **(context or {})
    }

    if severity == "CRITICAL":
        logger.critical(
            f"CRITICAL ALERT: {error_type} in {module}",
            **extra
        )
    else:
        logger.error(
            f"{error_type} in {module}: {str(error)}",
            **extra
        )

    errors_by_type.labels(module=module, error_type=error_type).inc()


# ============================================================================
# Decorator for Automatic Instrumentation
# ============================================================================

def traced(phase: str, operation: Optional[str] = None):
    """Decorator to automatically trace function execution.

    Usage:
        @traced("embed", "embed_batch")
        def embed_batch(self, texts):
            ...
    """
    def decorator(func: Callable):
        @wraps(func)
        def wrapper(*args, **kwargs):
            op = operation or func.__name__
            request_id = get_request_id() or "no-request"

            log = get_logger(func.__module__)
            log.debug(
                f"Starting {phase}.{op}",
                function=func.__name__,
                request_id=request_id
            )

            start_time = time.perf_counter()
            try:
                result = func(*args, **kwargs)
                duration = time.perf_counter() - start_time

                log.info(
                    f"Completed {phase}.{op}",
                    function=func.__name__,
                    request_id=request_id,
                    duration_ms=round(duration * 1000, 2)
                )
                return result
            except Exception as e:
                duration = time.perf_counter() - start_time
                log.error(
                    f"Failed {phase}.{op}",
                    function=func.__name__,
                    request_id=request_id,
                    duration_ms=round(duration * 1000, 2),
                    error=str(e),
                    error_type=get_error_type(e)
                )
                raise

        return wrapper
    return decorator


# ============================================================================
# Metrics Export Endpoint
# ============================================================================

def get_metrics_handler():
    """Return a WSGI app for Prometheus metrics scraping."""
    return make_wsgi_app()


# ============================================================================
# Metrics Server (runs in a separate thread)
# ============================================================================

class ThreadedWSGIServer(ThreadingMixIn, WSGIServer):
    """Threaded WSGI server for serving metrics."""
    daemon_threads = True


def start_metrics_server(port: int = 9090) -> Thread:
    """Start a background thread serving Prometheus metrics.

    Args:
        port: Port number for the metrics server (default 9090).

    Returns:
        The started Thread.
    """
    app = make_wsgi_app()

    def serve():
        httpd = make_server("0.0.0.0", port, app, ThreadedWSGIServer)
        httpd.serve_forever()

    t = Thread(target=serve, daemon=True, name="metrics-server")
    t.start()
    return t


# ============================================================================
# Log file viewer helper
# ============================================================================

def read_recent_logs(log_file: str = "logs/app.log", lines: int = 100) -> str:
    """Read the most recent log entries.

    Args:
        log_file: Path to the log file.
        lines: Number of recent lines to return.

    Returns:
        Last N lines of the log file as a string.
    """
    try:
        with open(log_file, "r") as f:
            all_lines = f.readlines()
            return "".join(all_lines[-lines:])
    except FileNotFoundError:
        return f"Log file not found: {log_file}"
    except Exception as e:
        return f"Error reading log file: {e}"
