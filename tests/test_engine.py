"""Tests for hippmem Engine Python binding."""

import tempfile
import os
import pytest
from hippmem import Engine


@pytest.fixture
def engine():
    """Create an engine with a temporary store."""
    with tempfile.TemporaryDirectory() as tmpdir:
        store_path = os.path.join(tmpdir, "test.redb")
        e = Engine.open(store_path)
        yield e
        e.close()


def test_open_and_close(engine):
    """Engine opens and closes without error."""
    assert engine is not None


# ── embedder parameter (D10) ──

def test_open_explicit_hash_embedder():
    """embedder='hash' is explicit and works."""
    with tempfile.TemporaryDirectory() as tmpdir:
        e = Engine.open(os.path.join(tmpdir, "t.redb"), embedder="hash")
        e.write("test")
        e.close()


def test_open_neural_requires_all_params():
    """embedder='neural' without api params raises TypeError."""
    with tempfile.TemporaryDirectory() as tmpdir:
        path = os.path.join(tmpdir, "t.redb")
        with pytest.raises(TypeError):
            Engine.open(path, embedder="neural")
        with pytest.raises(TypeError):
            Engine.open(path, embedder="neural", api_key="sk-x")
        with pytest.raises(TypeError):
            Engine.open(
                path,
                embedder="neural",
                api_base_url="https://api.openai.com/v1",
                api_key="sk-x",
            )


def test_open_unknown_embedder_raises():
    """Unknown embedder name raises ValueError."""
    with tempfile.TemporaryDirectory() as tmpdir:
        with pytest.raises(ValueError):
            Engine.open(os.path.join(tmpdir, "t.redb"), embedder="magic")


def test_open_neural_full_params_builds():
    """embedder='neural' with all params constructs the embedder (may fail on auth at embed time)."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Construction succeeds; embedding without a real key fails gracefully later.
        e = Engine.open(
            os.path.join(tmpdir, "t.redb"),
            embedder="neural",
            api_base_url="https://api.openai.com/v1",
            api_key="sk-test-invalid",
            model="text-embedding-3-small",
        )
        # write triggers an embedding; with an invalid key it should error, not panic.
        try:
            e.write("hello")
        except Exception:
            pass
        e.close()


# ── feedback / retrieval_id (D11: usage feedback loop) ──

def test_retrieve_exposes_retrieval_id(engine):
    """RetrieveOutput carries a retrieval_id for feedback."""
    engine.write("The user prefers Rust.")
    results = engine.retrieve("What does the user prefer?")
    assert isinstance(results.retrieval_id, int)
    assert results.retrieval_id > 0


def test_feedback_accepts_valid_signals(engine):
    """All four usage signals are accepted."""
    engine.write("The user prefers Rust.")
    results = engine.retrieve("What does the user prefer?")
    used = [r.memory_id for r in results.results]
    for signal in ["referenced", "user_confirmed_correct", "task_succeeded", "user_rejected"]:
        engine.feedback(results.retrieval_id, used, signal)


def test_feedback_unknown_signal_raises(engine):
    """Unknown signal name raises ValueError."""
    engine.write("The user prefers Rust.")
    results = engine.retrieve("What does the user prefer?")
    with pytest.raises(ValueError):
        engine.feedback(results.retrieval_id, [], "bogus_signal")


# ── consolidate ──

def test_consolidate_incremental(engine):
    """Incremental consolidation returns a report."""
    engine.write("The user prefers Rust.", content_type="Preference")
    report = engine.consolidate()
    assert report.memories_processed >= 1
    assert report.elapsed_ms >= 0


def test_consolidate_valid_scopes(engine):
    """All documented scopes are accepted."""
    engine.write("The user prefers Rust.")
    for scope in ["incremental", "full", "edges_only", "reindex"]:
        report = engine.consolidate(scope)
        assert report.elapsed_ms >= 0


def test_consolidate_unknown_scope_raises(engine):
    """Unknown scope name raises ValueError."""
    with pytest.raises(ValueError):
        engine.consolidate("bogus")


def test_write_single(engine):
    """Writing a memory returns expected output."""
    out = engine.write("The user prefers Rust.", content_type="Preference", importance=0.8)
    assert out.memory_id
    assert int(out.memory_id) > 0  # memory_id is a numeric string (u128)
    assert out.stage
    assert out.links_count == 0  # first memory has nothing to link to


def test_write_multiple(engine):
    """Multiple writes create associations."""
    out1 = engine.write("The user likes Rust.", content_type="Preference")
    out2 = engine.write("The user chose redb for embedded storage.", content_type="Decision")
    # Second write may have links to the first
    assert out2.links_count >= 0


def test_retrieve_finds_results(engine):
    """Retrieval returns results after writing."""
    engine.write("The user prefers Rust for backend development.")
    results = engine.retrieve("What language does the user prefer?", top_k=3)
    assert len(results.results) > 0
    assert results.latency_ms >= 0


def test_retrieve_result_structure(engine):
    """Each retrieval result has the expected fields."""
    engine.write("The user thinks PostgreSQL is great for OLTP workloads.", content_type="Preference", importance=0.9)
    results = engine.retrieve("PostgreSQL", top_k=3)
    assert len(results.results) > 0

    r = results.results[0]
    assert r.memory_id
    assert 0.0 <= r.score <= 1.0
    assert len(r.content) > 0
    assert r.content_type
    assert isinstance(r.dimensions, list)


def test_retrieve_with_hops(engine):
    """Retrieval with explicit max_hops works."""
    engine.write("Memory A")
    engine.write("Memory B related to A")
    results = engine.retrieve("Memory A", top_k=3, max_hops=2)
    assert results.hops_used >= 0


def test_custom_content_types(engine):
    """All supported content types work."""
    for ct in ["Decision", "Preference", "ProjectKnowledge", "Correction", "Event"]:
        out = engine.write(f"Test memory with {ct}", content_type=ct)
        assert out.memory_id


def test_default_values(engine):
    """Default parameters work."""
    out = engine.write("A simple memory.")
    assert out.memory_id
    results = engine.retrieve("simple memory")
    assert results.latency_ms >= 0
