"""
HIPPMEM — native associative memory engine for AI agents.
"""

# The native extension is installed alongside this package by maturin.
# It may be named _hippmem or hippmem depending on build config.
try:
    from _hippmem import Engine, WriteOutput, RetrievalResult, RetrieveOutput
except ImportError:
    from hippmem import Engine, WriteOutput, RetrievalResult, RetrieveOutput

__all__ = ["Engine", "WriteOutput", "RetrievalResult", "RetrieveOutput"]
