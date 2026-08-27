# Changelog

## [0.4.4] — 2026-08-27

- **Added**: `delete_memories` (full cascade cleanup: storage, indexes, edges, context links), `list_memories` (paginated, content-type filter), `export_memories` (JSONL full export)
- **Changed**: `extractor` backend parameter (`hash` / `neural` / `auto`, default `auto` — OpenAI-compatible extraction when a key is present); `embedder` defaults to `auto` (neural when `OPENAI_API_KEY` is available, deterministic hash otherwise)
- Extractor settings are independent from the embedder: `HIPPMEM_EXTRACTOR_API_KEY` / `HIPPMEM_EXTRACTOR_BASE_URL` / `HIPPMEM_EXTRACTOR_MODEL`

## [0.4.3] — 2026-08-21

- **Changed**: engine 0.4.3 alignment — confirmation learns context-answer links (a confirmed memory lifts only in queries sharing its entity/topic context, accumulating across rounds), propagation-path edges strengthen on confirmation, and edge decay follows a forgetting curve (review-dependent half-life). Retrieval improves with feedback as intended.

## [0.4.2] — 2026-08-19
 — 2026-08-19

- **Added**: `RetrieveOutput.semantic_index_degraded` — reports when the store has memories but no persisted dense vectors (old store / embedding-failed store); run `consolidate("reindex")` to rebuild the dense index
- **Changed**: engine 0.4.2 alignment — semantic indexes are persisted and rebuilt on open, so semantic recall survives restarts (previously every reopened store silently lost SemanticDense/SemanticBinary)

## [0.4.1] — 2026-08-18
 — 2026-08-18

- **Changed**: engine 0.4.1 alignment — multi-entity queries ("what is the relationship between X and Y?") now prefer memories covering more of the query's entities (entity channel tiers 0.2/0.35/0.5 + a rerank multiplier of up to 1.2·k/N), so full-coverage answers overtake single-entity word-surface decoys; single-entity queries are unaffected
- **Fixed**: Chinese entity extraction no longer fuses a proper name with the copula (the fused "Li Hua shi" form no longer breaks canonical-exact entity matching); empty `user_rejected` feedback is now a retrieval-quality signal with no memory-side effects (audit only)

## [0.4.0] — 2026-08-12
 — 2026-08-12

- **Fixed**: retrieval is now deterministic — identical store state and query
  produce bit-identical scores; consolidation no longer duplicates
  co-activation edges across cycles; edge changes made during consolidation
  now actually reach retrieval (written to the graph store)
- **Changed**: summaries and their compressed sources no longer hit the
  retrieval channels directly — a summary's text is a concatenation of its
  sources, so it used to outrank the concrete, correct memories in every
  related query; sources stay inspectable and a drill-down path is planned
- **Changed**: retrieval no longer reinforces itself — merely running a query
  no longer boosts the memories it returns; only explicit feedback does.
  Confirmed memories no longer score higher in every query: feedback works
  through association edges, scaled by signal strength
- **Changed**: `user_rejected` semantics — a targeted rejection (non-empty
  used ids) weakens the rejected memories' association edges during the next
  consolidation; an empty list rejects the whole result set (its memories are
  suppressed in the recency channel). Rejections never strengthen anything

## [0.3.0] — 2026-08-10

- **Fixed**: feedback had no observable effect on retrieval — memory ids were
  truncated in the engine's activation log; ids are now stored in full, so
  confirmations genuinely strengthen memories and rejections lower their usage
  score (rejected memories are never boosted by the recent-activity channel)
- **New**: summaries created by `consolidate()` are now searchable (previously
  invisible to all recall channels); source memories are marked compressed and
  hidden from retrieval results
- **Changed**: summarization triggers per cluster of similar low-importance
  memories instead of over the whole store; memories already covered by a
  summary are not re-summarized
- **New**: retrieval traces report real `hops_used`; `max_hops` is honored by
  graph traversal
- Depends on hippmem-engine 0.3.0

## [0.2.1] — 2026-08-07

- **New**: `engine.feedback(retrieval_id, used_memory_ids, signal)` — usage
  feedback for Hebbian learning (retrieval gets more accurate with use)
- **New**: `RetrieveOutput.retrieval_id` — identifier for feedback
- **New**: `engine.consolidate(scope="incremental")` — run Hebbian → decay →
  compaction → summary; returns `ConsolidationReport`
- Depends on hippmem-engine 0.2.1 (retrieval_id support)

## [0.2.0] — 2026-08-05

- **New**: `Engine.open(embedder="neural", api_base_url=..., api_key=..., model=...)`
  — Neural embedder for higher semantic accuracy
- **New**: `embedder` parameter — `"hash"` (default, offline) or `"neural"` (API)
- **Hard constraint**: `embedder="neural"` requires all three of
  `api_base_url`, `api_key`, `model` — missing any raises `TypeError`
- Unknown embedder name raises `ValueError`
- Bumped to 0.2.0 (aligned with hippmem-engine 0.2.0)

## [0.1.1] — 2026-07-26

- Add docstrings to all classes and methods (runtime `help()` support)
- Add `.pyi` type stub for VSCode/Pylance autocomplete

## [0.1.0] — 2026-07-25

- Initial release: Python bindings for HIPPMEM
- `Engine.open()` / `engine.write()` / `engine.retrieve()` / `engine.close()`
- Deterministic fallback backend (no API key required)
- Content type parsing: Decision, Preference, ProjectKnowledge, etc.
- Multi-channel seed recall with spreading activation
- pip install hippmem
