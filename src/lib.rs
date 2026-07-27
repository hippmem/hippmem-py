use hippmem_core::model::enums::ContentType;
use hippmem_core::model::links::RetrievalMode;
use hippmem_core::model::unit::WriteContext;
use hippmem_engine::{Engine, EngineConfig, RetrieveContext, RetrieveInput, WriteMemoryInput};
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
    /// Ranked retrieval results.
    #[pyo3(get)]
    results: Vec<RetrievalResult>,
    /// Query latency in milliseconds.
    #[pyo3(get)]
    latency_ms: u32,
    /// Graph traversal hops used.
    #[pyo3(get)]
    hops_used: u8,
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
    ///
    /// Returns:
    ///     An Engine instance connected to the store.
    #[staticmethod]
    #[pyo3(signature = (path = None))]
    fn open(path: Option<String>) -> PyResult<Self> {
        let store_dir = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./hippmem_data"));

        let engine = Engine::open(EngineConfig {
            store_dir,
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
            results,
            latency_ms: out.diagnostics.latency_ms,
            hops_used: out.trace.hops_used,
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
