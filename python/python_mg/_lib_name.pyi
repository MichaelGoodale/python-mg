import datetime
from typing import Literal, Sequence
import numpy as np
import numpy.typing as npt

from python_mg.syntax import ParseTree

class MGNode:
    def is_trace(self) -> bool: ...
    def trace_id(self) -> int: ...
    def lemma_string(self) -> str: ...
    def is_stolen(self) -> str: ...

class MGEdge:
    def is_move(self) -> bool: ...
    def is_head_move(self) -> bool: ...
    def is_merge(self) -> bool: ...

class SyntacticStructure:
    """A parse tree for some string."""

    def __init__(self) -> None: ...
    def pronunciation(self) -> list[str]: ...
    def log_prob(self) -> float: ...
    def n_steps(self) -> int: ...
    def contains_lexical_entry(self, s: str) -> bool: ...
    def contains_word(self, s: str | None) -> bool: ...
    def prob(self) -> float: ...
    def latex(self) -> str: ...
    def to_tree(self) -> ParseTree: ...
    def max_memory_load(self) -> int: ...
    def tokens(self) -> npt.NDArray[np.uint]: ...
    @property
    def meaning(self) -> list[Meaning] | None: ...
    def __to_tree_inner(
        self,
    ) -> tuple[list[tuple[int, MGNode]], list[tuple[int, int, MGEdge]], int]: ...

class Continuation:
    """A continuation of a prefix string."""

    def __init__(self, word: str) -> None: ...
    @staticmethod
    def EOS() -> "Continuation": ...
    def is_end_of_string(self) -> bool: ...
    def is_word(self) -> bool: ...
    def is_multi_word(self) -> bool: ...

class GrammarIterator:
    def __iter__(self) -> GrammarIterator: ...
    def __next__(self) -> SyntacticStructure: ...

class Lexicon:
    """A Minimalist Grammar Lexicon."""

    def __init__(self, s: str) -> None: ...
    @staticmethod
    def random_lexicon(lemmas: list[str]) -> "Lexicon": ...
    def mdl(self, n_phonemes: int) -> float: ...
    def is_semantic(self) -> bool: ...
    def continuations(
        self,
        prefix: str,
        category: str,
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> set[Continuation]: ...
    def generate_unique_strings(
        self,
        category: str,
        min_log_prob: float = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> list[tuple[list[str], float]]: ...
    def generate_grammar(
        self,
        category: str,
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> GrammarIterator: ...
    def parse(
        self,
        s: str,
        category: str,
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> list[SyntacticStructure]: ...
    def parse_tokens(
        self,
        s: Sequence[int] | npt.NDArray[np.uint],
        category: str,
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> list[SyntacticStructure]: ...
    def tokens(self) -> dict[str, int]: ...
    def detokenize(self, s: Sequence[int] | npt.NDArray[np.uint]) -> list[str]: ...
    def detokenize_batch(
        self,
        s: Sequence[Sequence[int]] | list[npt.NDArray[np.uint]] | npt.NDArray[np.uint],
    ) -> list[list[str]]: ...
    def token_continuations(
        self,
        s: npt.NDArray[np.uint],
        category: str,
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
    ) -> npt.NDArray[np.bool]: ...

class Actor:
    name: str
    properties: set[str]

    def __init__(
        self,
        name: str,
        properties: set[str] | None = None,
    ) -> None: ...

class Event:
    agent: str | None
    patient: str | None
    properties: set[str]

    def __init__(
        self,
        agent: str | None = None,
        patient: str | None = None,
        properties: set[str] | None = None,
    ) -> None: ...

class Meaning:
    def __init__(self, s: str) -> None: ...
    def bind_free_variable(
        self, free_var: str | int, value: Meaning | str, reduce: bool = True
    ) -> Meaning: ...
    def apply(self, psi: Meaning | str, reduce: bool = True) -> Meaning: ...
    def reduce(self) -> Meaning: ...

class PossibleEvent:
    has_agent: bool
    has_patient: bool
    is_reflexive: bool
    name: str

    def __init__(
        self,
        name: str,
        has_agent: bool = True,
        has_patient: bool = False,
        is_reflexive: bool = True,
    ) -> None: ...
    def event_kind(self) -> Literal[
        "Transitive",
        "TransitiveNonReflexive",
        "Unergative",
        "Unaccusative",
        "Avalent",
    ]: ...

class Scenario:
    actors: list[Actor]
    events: list[Event]
    questions: list[Meaning]

    def __init__(
        self, actors: list[Actor], events: list[Event], questions: list[Meaning | str]
    ) -> None: ...
    @staticmethod
    def from_str(s: str) -> Scenario: ...
    def evaluate(
        self,
        expression: str | Meaning,
        max_steps: int | None = 256,
        timeout: datetime.timedelta | None = None,
    ) -> bool | Actor | Event | set[Actor] | set[Event]: ...
    @staticmethod
    def all_scenarios(
        actors: list[str],
        event_kinds: list[PossibleEvent],
        actor_properties: list[str],
        max_number_of_events: None | int = None,
        max_number_of_actors: None | int = None,
        max_number_of_actor_properties: None | int = None,
    ) -> ScenarioGenerator: ...

class ScenarioGenerator:
    def __iter__(self) -> ScenarioGenerator: ...
    def __next__(self) -> Scenario: ...
