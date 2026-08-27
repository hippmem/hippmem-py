use hippmem_core::config::EmbedderConfig;
use hippmem_core::model::enums::ContentType;
use hippmem_core::model::links::RetrievalMode;
use hippmem_core::model::unit::WriteContext;
use hippmem_engine::{Engine, EngineConfig, RetrieveContext, RetrieveInput, WriteMemoryInput};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use std::path::PathBuf;

// ── Python types ──

/// Result of writing a memory.
#[pyclass]
#[derive(Clone)]
struct WriteOutput {
    /// Unique memory identifier.
    #[pyo3(get)]
    memory_id: String,
    /// Processing stage reached.
    #[pyo3(get)]
    stage: String,
    /// Number of associations created with existing memories.
    #[pyo3(get)]
    links_count: usize,
}

/// A single retrieval result from a memory query.
#[pyclass]
#[derive(Clone)]
struct RetrievalResult {
    /// Unique memory identifier.
    #[pyo3(get)]
    memory_id: String,
    /// Relevance score (0.0–1.0).
    #[pyo3(get)]
    score: f32,
    /// Memory text content.
    #[pyo3(get)]
    content: String,
    /// Content type (Decision, Preference, ProjectKnowledge, etc.).
    #[pyo3(get)]
    content_type: String,
    /// Matched dimensions (e.g., Entity, Semantic, Causal).
    #[pyo3(get)]
    dimensions: Vec<String>,
}

/// Result of a retrieval query.
#[pyclass]
#[derive(Clone)]
struct RetrieveOutput {
    /// Identifier for this retrieval — pass to `Engine.feedback`.
    #[pyo3(get)]
    retrieval_id: u64,
    /// Ranked retrieval results.
    #[pyo3(get)]
    results: Vec<RetrievalResult>,
    /// Query latency in milliseconds.
    #[pyo3(get)]
    latency_ms: u32,
    /// Graph traversal hops used.
    #[pyo3(get)]
    hops_used: u8,
    /// True when the store has memories but no persisted dense vectors
    /// (old store / embedding-failed store): SemanticDense recall is empty.
    /// Run `consolidate("reindex")` to rebuild the dense index.
    #[pyo3(get)]
    semantic_index_degraded: bool,
}

/// A single memory in a list result.
#[pyclass]
#[derive(Clone)]
struct ListItem {
    #[pyo3(get)]
    memory_id: String,
    #[pyo3(get)]
    content_preview: String,
    #[pyo3(get)]
    content_type: String,
    #[pyo3(get)]
    created_at_ms: i64,
    #[pyo3(get)]
    importance: f32,
}

/// Paginated list result.
#[pyclass]
#[derive(Clone)]
struct ListOutput {
    #[pyo3(get)]
    items: Vec<ListItem>,
    /// Cursor for the next page; None means the last page.
    #[pyo3(get)]
    next_cursor: Option<String>,
    #[pyo3(get)]
    total: u64,
}

/// Report of a consolidation run.
#[pyclass]
#[derive(Clone)]
struct ConsolidationReport {
    /// Memories processed.
    #[pyo3(get)]
    memories_processed: u64,
    /// Edges decayed (strength × decay).
    #[pyo3(get)]
    edges_decayed: u64,
    /// Weak edges archived.
    #[pyo3(get)]
    edges_archived: u64,
    /// Edges merged.
    #[pyo3(get)]
    edges_merged: u64,
    /// Summaries created.
    #[pyo3(get)]
    summaries_created: u64,
    /// Contradictions found.
    #[pyo3(get)]
    contradictions_found: u64,
    /// Elapsed milliseconds.
    #[pyo3(get)]
    elapsed_ms: u64,
}

// ── Engine wrapper ──

/// HIPPMEM memory engine.
///
/// Open or create a memory store, write memories, and retrieve them
/// via multi-channel associative recall.
///
/// The engine discovers associations (entity, causal, semantic, topic,
/// temporal) at write time and retrieves via spreading activation —
/// remembering WHY, not just WHAT.
///
/// No GPU, API key, or network required — uses a deterministic
/// fallback backend by default.
///
/// Usage::
///
///     engine = Engine.open()
///     engine.write("The user prefers Rust.", content_type="Preference")
///     results = engine.retrieve("What language does the user prefer?")
///     for r in results.results:
///         print(f"[{r.score:.3f}] {r.content}")
#[pyclass(name = "Engine")]
struct PyEngine {
    inner: Engine,
}

#[pymethods]
impl PyEngine {
    /// Open or create a HIPPMEM memory store.
    ///
    /// Args:
    ///     path: Path to the store file. Defaults to ``"./hippmem_data"``.
    ///     embedder: ``"auto"`` (default — uses ``neural`` when
    ///         ``OPENAI_API_KEY`` is present, ``hash`` otherwise),
    ///         ``"hash"`` (offline SimHash) or ``"neural"`` (API-based,
    ///         requires api_base_url/api_key/model).
    ///     extractor: ``"auto"`` (default — uses ``neural`` when
    ///         ``HIPPMEM_EXTRACTOR_API_KEY``/``OPENAI_API_KEY`` is present,
    ///         deterministic rules otherwise), ``"hash"`` (deterministic
    ///         rules only) or ``"neural"`` (OpenAI-compatible structured
    ///         extraction, requires the extractor env config).
    ///     api_base_url: Required when ``embedder="neural"``.
    ///     api_key: Required when ``embedder="neural"``.
    ///     model: Required when ``embedder="neural"``.
    ///
    /// Returns:
    ///     An Engine instance connected to the store.
    ///
    /// Raises:
    ///     TypeError: ``embedder="neural"`` without all three API params.
    ///     (``embedder="auto"`` reads OPENAI_API_KEY / HIPPMEM_EMBEDDING_* env.)
    ///     ValueError: unknown embedder name.
    #[staticmethod]
    #[pyo3(signature = (
        path = None,
        embedder = "auto",
        extractor = "auto",
        api_base_url = None,
        api_key = None,
        model = None,
    ))]
    fn open(
        path: Option<String>,
        embedder: &str,
        extractor: &str,
        api_base_url: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
    ) -> PyResult<Self> {
        let store_dir = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./hippmem_data"));

        let embedder_config = match embedder {
            // Auto: strongest-first default (2026-08-26) — neural when
            // OPENAI_API_KEY is set, deterministic hash otherwise.
            "auto" => EmbedderConfig::Auto,
            "hash" => EmbedderConfig::Hash { dimensions: 256 },
            "neural" => {
                let (Some(base_url), Some(key), Some(model_name)) = (api_base_url, api_key, model)
                else {
                    return Err(PyTypeError::new_err(
                        "embedder='neural' requires api_base_url, api_key, and model",
                    ));
                };
                EmbedderConfig::Neural {
                    base_url,
                    model: model_name,
                    api_key: Some(key),
                    dimensions: 1536,
                }
            }
            other => {
                return Err(PyValueError::new_err(format!(
                    "embedder must be 'auto', 'hash' or 'neural', got {other:?}"
                )));
            }
        };

        // Extractor backend choice (08 §5): "auto" uses the OpenAI-compatible
        // backend when HIPPMEM_EXTRACTOR_API_KEY / OPENAI_API_KEY is present,
        // deterministic rules otherwise — the default-enhancement /
        // degraded-guarantee semantics (vendor-neutral, 2026-08-26).
        let backend = hippmem_model::registry::BackendSelection {
            extractor: match extractor {
                "hash" => hippmem_model::registry::BackendChoice::Deterministic,
                "neural" => hippmem_model::registry::BackendChoice::Api,
                "auto" => hippmem_model::registry::BackendChoice::Auto,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "extractor must be 'hash', 'neural' or 'auto', got {other:?}"
                    )));
                }
            },
            ..Default::default()
        };

        let engine = Engine::open(EngineConfig {
            store_dir,
            embedder: embedder_config,
            backend,
            ..Default::default()
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyEngine { inner: engine })
    }

    /// Write a memory and discover associations.
    ///
    /// The engine extracts entities, topics, and causal links, then
    /// discovers associations (entity, temporal, semantic, topic,
    /// causal) with existing memories. Associations are stored as
    /// typed edges in the memory graph.
    ///
    /// Args:
    ///     content: Memory text to store.
    ///     content_type: One of ``"Decision"``, ``"Preference"``,
    ///         ``"ProjectKnowledge"``, ``"TaskState"``, ``"Correction"``,
    ///         ``"Event"``, ``"Reflection"``.
    ///     importance: 0.0–1.0 importance hint.
    ///
    /// Returns:
    ///     WriteOutput with memory ID and link count.
    #[pyo3(signature = (content, content_type = None, importance = None))]
    fn write(
        &self,
        content: &str,
        content_type: Option<&str>,
        importance: Option<f32>,
    ) -> PyResult<WriteOutput> {
        let ct = content_type
            .map(parse_content_type)
            .unwrap_or(ContentType::UserStatement);

        let input = WriteMemoryInput {
            content: content.to_string(),
            content_type: Some(ct),
            context: WriteContext {
                conversation_id: None,
                session_id: None,
                project_id: None,
                task_id: None,
                user_id: None,
                local_time: hippmem_core::time::Timestamp(0),
                preceding_memory_ids: vec![],
                source_refs: vec![],
            },
            importance_hint: importance,
            source_refs: vec![],
        };

        let out = self
            .inner
            .write(input)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(WriteOutput {
            memory_id: out.memory_id.0.to_string(),
            stage: format!("{:?}", out.stage_reached),
            links_count: out.created_links.len(),
        })
    }

    /// Retrieve memories via multi-channel associative recall.
    ///
    /// Uses 5 recall channels (BM25, entity inverted index, semantic
    /// dense, semantic binary, topic cluster) fused by RRF, then
    /// spreads activation over the association graph.
    ///
    /// Unlike keyword search, associative retrieval finds memories
    /// connected by entity, causal, and topic relationships —
    /// surfacing WHY, not just WHAT.
    ///
    /// Args:
    ///     query: Natural-language search query.
    ///     top_k: Maximum number of results (default 5).
    ///     max_hops: Graph traversal depth. None = auto.
    ///
    /// Returns:
    ///     RetrieveOutput with ranked results, latency, and hop count.
    #[pyo3(signature = (query, top_k = 5, max_hops = None))]
    fn retrieve(
        &self,
        query: &str,
        top_k: usize,
        max_hops: Option<usize>,
    ) -> PyResult<RetrieveOutput> {
        let input = RetrieveInput {
            query: query.to_string(),
            context: RetrieveContext::default(),
            top_k,
            max_hops,
            retrieval_mode: RetrievalMode::Balanced,
        };

        let out = self
            .inner
            .retrieve(input)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let results: Vec<RetrievalResult> = out
            .results
            .iter()
            .map(|r| RetrievalResult {
                memory_id: r.memory.id.0.to_string(),
                score: r.final_score,
                content: r.memory.content.raw.clone(),
                content_type: format!("{:?}", r.memory.content.content_type),
                dimensions: r
                    .matched_dimensions
                    .iter()
                    .map(|d| format!("{:?}", d))
                    .collect(),
            })
            .collect();

        Ok(RetrieveOutput {
            retrieval_id: out.retrieval_id,
            results,
            latency_ms: out.diagnostics.latency_ms,
            hops_used: out.trace.hops_used,
            semantic_index_degraded: self.inner.semantic_index_degraded(),
        })
    }

    /// Send usage feedback for a previous retrieval.
    ///
    /// The engine uses this signal for Hebbian learning — memories that
    /// were actually used get strengthened (via usage score and edge
    /// reinforcement), rejected ones get a lower usage score and are never
    /// boosted by the recent-activity channel. The more feedback you
    /// provide, the more accurate retrieval becomes.
    ///
    /// Args:
    ///     retrieval_id: From ``RetrieveOutput.retrieval_id``.
    ///     used_memory_ids: Memory IDs (as returned in results) that were
    ///         actually used/confirmed.
    ///     signal: One of ``"referenced"``, ``"user_confirmed_correct"``,
    ///         ``"task_succeeded"``, ``"user_rejected"``.
    ///
    /// Raises:
    ///     ValueError: unknown signal name.
    #[pyo3(signature = (retrieval_id, used_memory_ids, signal))]
    fn feedback(
        &self,
        retrieval_id: u64,
        used_memory_ids: Vec<String>,
        signal: &str,
    ) -> PyResult<()> {
        use hippmem_engine::UsageSignal;

        let usage_signal = match signal {
            "referenced" => UsageSignal::Referenced,
            "user_confirmed_correct" => UsageSignal::UserConfirmedCorrect,
            "task_succeeded" => UsageSignal::TaskSucceeded,
            "user_rejected" => UsageSignal::UserRejected,
            other => {
                return Err(PyValueError::new_err(format!(
                    "signal must be one of 'referenced', 'user_confirmed_correct', \
                     'task_succeeded', 'user_rejected', got {other:?}"
                )));
            }
        };

        let ids: Vec<hippmem_core::ids::MemoryId> = used_memory_ids
            .iter()
            .filter_map(|s| s.parse::<u128>().ok())
            .map(hippmem_core::ids::MemoryId)
            .collect();

        self.inner
            .feedback(hippmem_engine::FeedbackInput {
                retrieval_id,
                used_memory_ids: ids,
                signal: usage_signal,
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(())
    }

    /// Run consolidation: Hebbian learning → decay → compaction → summary.
    ///
    /// Consolidation evolves the memory graph: frequently co-activated
    /// connections strengthen, stale edges decay, weak edges are archived,
    /// and clusters of similar low-importance memories are summarized into a
    /// summary memory (the sources are marked compressed and hidden from
    /// retrieval results). Call it periodically (e.g. at session end) to keep
    /// retrieval accurate over time.
    ///
    /// Args:
    ///     scope: ``"incremental"`` (default), ``"full"``, ``"edges_only"``,
    ///         or ``"reindex"`` (rebuild all secondary indexes).
    ///
    /// Returns:
    ///     ConsolidationReport with processed counts and elapsed time.
    ///
    /// Raises:
    ///     ValueError: unknown scope name.
    /// List memories (paginated, optional content-type filter).
    ///
    /// Args:
    ///     limit: Page size (default 20, max 100).
    ///     cursor: Opaque cursor string from the previous page (None = first page).
    ///     content_type: Optional filter (Decision, Preference, ...).
    /// Export all memories as JSONL (memory + associations).
    ///
    /// Args:
    ///     output_path: Optional file path to write the export to; None
    ///         returns the JSONL string.
    ///
    /// Returns:
    ///     dict with count, json (when no path) and written_to.
    #[pyo3(signature = (limit = 20, cursor = None, content_type = None))]
    fn list_memories(
        &self,
        limit: usize,
        cursor: Option<String>,
        content_type: Option<&str>,
    ) -> PyResult<ListOutput> {
        let out = self
            .inner
            .list(hippmem_engine::ListInput {
                limit: limit.min(100),
                cursor: cursor.and_then(|c| c.parse::<u128>().ok()),
                content_type: content_type.map(parse_content_type),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(ListOutput {
            items: out
                .items
                .into_iter()
                .map(|i| ListItem {
                    memory_id: i.id.0.to_string(),
                    content_preview: i.content_preview,
                    content_type: format!("{:?}", i.content_type),
                    created_at_ms: i.created_at.0,
                    importance: i.importance,
                })
                .collect(),
            next_cursor: out.next_cursor.map(|c| c.to_string()),
            total: out.total,
        })
    }

    /// Export all memories as JSONL (memory + associations).
    ///
    /// Args:
    ///     output_path: Optional file path to write the export to; None
    ///         returns the JSONL string.
    ///
    /// Returns:
    ///     dict with count, json (when no path) and written_to.
    #[pyo3(signature = (output_path = None))]
    fn export_memories(
        &self,
        output_path: Option<String>,
    ) -> PyResult<pyo3::Py<pyo3::types::PyDict>> {
        use pyo3::types::PyDict;
        let out = self
            .inner
            .dump(hippmem_engine::DumpInput {
                output_path: output_path.map(PathBuf::from),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Python::with_gil(|py| {
            let d = PyDict::new(py);
            d.set_item("count", out.count)?;
            if let Some(json) = out.json {
                d.set_item("json", json)?;
            }
            if let Some(written_to) = out.written_to {
                d.set_item("written_to", written_to.to_string_lossy().to_string())?;
            }
            Ok(d.unbind())
        })
    }

    /// Delete memories and their full cascade (kv, indexes, graph edges,
    /// context links). Audit records are kept.
    ///
    /// Args:
    ///     memory_ids: List of memory ids (decimal strings from write/list).
    ///
    /// Returns:
    ///     dict with deleted (count) and edges_removed.
    #[pyo3(signature = (memory_ids))]
    fn delete_memories(&self, memory_ids: Vec<String>) -> PyResult<pyo3::Py<pyo3::types::PyDict>> {
        use pyo3::types::PyDict;
        let ids: Result<Vec<_>, _> = memory_ids
            .iter()
            .map(|s| {
                s.parse::<u128>()
                    .map(hippmem_core::ids::MemoryId)
                    .map_err(|_| {
                        PyValueError::new_err(format!(
                            "memory_id must be a decimal integer, got {s:?}"
                        ))
                    })
            })
            .collect();
        let out = self
            .inner
            .delete(hippmem_engine::DeleteInput { memory_ids: ids? })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Python::with_gil(|py| {
            let d = PyDict::new(py);
            d.set_item("deleted", out.deleted)?;
            d.set_item("edges_removed", out.edges_removed)?;
            Ok(d.unbind())
        })
    }

    #[pyo3(signature = (scope = "incremental"))]
    fn consolidate(&self, scope: &str) -> PyResult<ConsolidationReport> {
        use hippmem_engine::ConsolidationScope;

        let scope_enum = match scope {
            "incremental" => ConsolidationScope::Incremental,
            "full" => ConsolidationScope::Full,
            "edges_only" => ConsolidationScope::EdgesOnly,
            "reindex" => ConsolidationScope::Reindex,
            other => {
                return Err(PyValueError::new_err(format!(
                    "scope must be one of 'incremental', 'full', 'edges_only', \
                     'reindex', got {other:?}"
                )));
            }
        };

        let report = self
            .inner
            .consolidate(scope_enum)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(ConsolidationReport {
            memories_processed: report.memories_processed,
            edges_decayed: report.edges_decayed,
            edges_archived: report.edges_archived,
            edges_merged: report.edges_merged,
            summaries_created: report.summaries_created,
            contradictions_found: report.contradictions_found,
            elapsed_ms: report.elapsed_ms,
        })
    }

    /// Flush the full-text index to disk.
    ///
    /// The underlying store is closed automatically when the Engine
    /// object is garbage collected.
    fn close(&self) -> PyResult<()> {
        self.inner.flush_fulltext();
        Ok(())
    }
}

// ── Module ──

#[pymodule]
fn hippmem(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEngine>()?;
    m.add_class::<WriteOutput>()?;
    m.add_class::<RetrievalResult>()?;
    m.add_class::<RetrieveOutput>()?;
    m.add_class::<ConsolidationReport>()?;
    Ok(())
}

// ── Helpers ──

fn parse_content_type(s: &str) -> ContentType {
    match s.to_lowercase().as_str() {
        "decision" => ContentType::Decision,
        "preference" => ContentType::Preference,
        "project_knowledge" | "projectknowledge" => ContentType::ProjectKnowledge,
        "task_state" | "taskstate" => ContentType::TaskState,
        "correction" => ContentType::Correction,
        "event" => ContentType::Event,
        "reflection" => ContentType::Reflection,
        "assistant_observation" | "assistantobservation" => ContentType::AssistantObservation,
        "tool_result" | "toolresult" => ContentType::ToolResult,
        _ => ContentType::UserStatement,
    }
}
