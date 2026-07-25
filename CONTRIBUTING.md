# Contributing to hippmem-py

Thanks for your interest in contributing! This document explains how to report
issues, submit changes, and follow the project conventions.

## Reporting Bugs

- Open an issue at <https://github.com/hippmem/hippmem-py/issues>.
- Include Python version (`python --version`), OS, `pip freeze | grep hippmem`,
  and a minimal reproduction.
- For issues in the underlying engine (detection, retrieval, association),
  the bug may be routed to [hippmem/hippmem](https://github.com/hippmem/hippmem).
- For security vulnerabilities, do **not** open a public issue — see
  [SECURITY.md](SECURITY.md).

## Submitting a Pull Request

1. Fork the repository.
2. Create a branch from `main`:
   ```bash
   git checkout -b my-fix
   ```
3. Make your changes. Keep PRs focused — one logical change per PR.
4. Ensure all checks pass (see [Development Setup](#development-setup)).
5. Commit using [Conventional Commits](https://www.conventionalcommits.org/):
   ```
   feat: add support for named entity filtering
   fix: correct memory_id type conversion for u128
   docs: clarify configuration backend options
   refactor: extract content_type parsing
   test: add retrieval with max_hops test
   chore: bump pyo3 to 0.24
   ```
6. Open a PR against `main` and describe the change, the motivation, and any
   trade-offs.

## Development Setup

```bash
git clone https://github.com/hippmem/hippmem-py.git
cd hippmem-py
python3 -m venv .venv && source .venv/bin/activate
pip install maturin pytest
maturin develop
pytest
```

## Code Style

Rust:
```bash
cargo fmt
cargo clippy -- -D warnings
```

Python:
```bash
pip install ruff
ruff format tests/
ruff check tests/
```

Conventions:
- Library code must not `unwrap`/`panic` on recoverable errors — return `Result`.
- Do not use `unsafe` without an approved ADR.
- Code comments are in English.
- Match the surrounding code's naming, density, and idioms.

## Commit Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <short imperative summary>

<optional body explaining why and what trade-offs>

<optional footer>
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`.

## Licensing

hippmem-py is Apache 2.0 (see [LICENSE](LICENSE) and [COPYRIGHT](COPYRIGHT)).

The underlying [hippmem-engine](https://crates.io/crates/hippmem-engine) is
AGPL-3.0-only. By contributing to hippmem-py, you agree that your contributions
will be licensed under Apache 2.0.

For commercial licensing inquiries regarding hippmem-engine, contact
hippmem@gmail.com.

## DCO (Developer Certificate of Origin)

Every commit must include a `Signed-off-by:` line certifying that you have the
right to submit the contribution:

```
feat: add named entity filtering

Signed-off-by: Your Name <you@example.com>
```

Add the line manually, or use `git commit -s` to append it automatically. By
signing off, you attest to the [Developer Certificate of Origin](https://developercertificate.org/)
v1.1.
