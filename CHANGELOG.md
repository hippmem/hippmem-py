# Changelog

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
