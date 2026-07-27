# Changelog

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
