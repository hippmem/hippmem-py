use hippmem_core::model::enums::ContentType;
use hippmem_core::model::links::RetrievalMode;
use hippmem_core::model::unit::WriteContext;
use hippmem_engine::{Engine, EngineConfig, RetrieveContext, RetrieveInput, WriteMemoryInput};
use pyo3::prelude::*;
use std::path::PathBuf;

// ── Python types ──

#[pyclass]
#[derive(Clone)]
struct WriteOutput {
    #[pyo3(get)]
    memory_id: String,
    #[pyo3(get)]
    stage: String,
    #[pyo3(get)]
    links_count: usize,
}

#[pyclass]
#[derive(Clone)]
struct RetrievalResult {
    #[pyo3(get)]
    memory_id: String,
    #[pyo3(get)]
    score: f32,
    #[pyo3(get)]
    content: String,
    #[pyo3(get)]
    content_type: String,
    #[pyo3(get)]
    dimensions: Vec<String>,
}

#[pyclass]
#[derive(Clone)]
struct RetrieveOutput {
    #[pyo3(get)]
    results: Vec<RetrievalResult>,
    #[pyo3(get)]
    latency_ms: u32,
    #[pyo3(get)]
    hops_used: u8,
}

// ── Engine wrapper ──

#[pyclass(name = "Engine")]
struct PyEngine {
    inner: Engine,
}

#[pymethods]
impl PyEngine {
    #[new]
    #[pyo3(signature = (path = None))]
    fn new(path: Option<String>) -> PyResult<Self> {
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

    fn close(&self) -> PyResult<()> {
        self.inner.flush_fulltext();
        Ok(())
    }
}

// ── Module ──

#[pymodule]
fn _hippmem(m: &Bound<'_, PyModule>) -> PyResult<()> {
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
