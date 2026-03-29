"""AutoResearch domain modules.

Each domain represents an autonomous research direction. The agent iterates
on these files to improve Provenance's capabilities. Domain modules expose
a standard interface:

    class DomainResearcher:
        def get_hypotheses() -> list[Hypothesis]
        def run_experiment(hypothesis, data, time_budget) -> ExperimentResult
        def get_current_baseline(data) -> float

The evaluation harness calls these methods. The agent modifies the
implementation to test new approaches.
"""

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class Hypothesis:
    """A research hypothesis to test."""
    id: str
    domain: str
    description: str
    approach: str
    expected_impact: str  # "high", "medium", "low"
    complexity: str       # "simple", "moderate", "complex"
    dependencies: list[str] = field(default_factory=list)


@dataclass
class ExperimentResult:
    """Result of running a single experiment."""
    hypothesis_id: str
    domain: str
    metric_before: float
    metric_after: float
    delta: float
    duration_seconds: float
    memory_mb: float
    status: str  # "improvement", "regression", "null", "error", "timeout"
    details: dict = field(default_factory=dict)
    notes: str = ""
    artifacts: list[str] = field(default_factory=list)


@dataclass
class DomainState:
    """Tracks the current state of research in a domain."""
    domain: str
    current_best_metric: float
    total_experiments: int = 0
    improvements: int = 0
    regressions: int = 0
    null_results: int = 0
    last_hypothesis: Optional[str] = None
    active_strategies: list[str] = field(default_factory=list)
