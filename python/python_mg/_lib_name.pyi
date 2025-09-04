from typing import Iterator, Optional

from python_mg.syntax import ParseTree

class MGNode:
    def is_trace(self) -> bool: ...
    def trace_id(self) -> int:
        """Gets the trace id of traces and raises an error otherwise"""

    def lemma_string(self) -> str:
        """Format the node as a string in a tree if leaf or trace"""

    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class MGEdge:
    def is_move(self) -> bool:
        """Checks whether the edge is a movement edge"""

    @staticmethod
    def move_edge() -> MGEdge:
        """Gets a movement edge"""

    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class SyntacticStructure:
    """A parse tree for some string"""

    def __init__(self) -> None: ...
    def log_prob(self) -> float:
        """Return the log probability."""

    def contains_lexical_entry(self, s: str) -> bool:
        """Check if this structure contains a specific lexical entry (formatted as an MG entry, will raise an error if unparseable)"""

    def contains_word(self, s: Optional[str]) -> bool:
        """Check if this structure contains a specific word."""

    def prob(self) -> float:
        """Return the probability of this syntactic structure."""

    def latex(self) -> str:
        """Return a LaTeX representation of this syntactic structure."""

    def to_tree(self) -> ParseTree:
        """Converts a syntactic structure into a graph structure"""

    def __to_tree_inner(
        self,
    ) -> tuple[list[tuple[int, MGNode]], list[tuple[int, int, MGEdge]], int]: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class Continuation:
    """A continuation of a prefix string"""

    def __init__(self) -> None: ...
    @staticmethod
    def EOS() -> "Continuation": ...
    def is_end_of_string(self) -> bool:
        """Check if the continuation is a end of string marker"""

    def is_word(self) -> bool:
        """Check if the continuation is a word"""

    def is_multi_word(self) -> bool:
        """Check if the continuation is an affixed word"""

    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class GrammarIterator(Iterator[SyntacticStructure]):
    def __iter__(self) -> GrammarIterator: ...
    def __next__(self) -> SyntacticStructure: ...

class Lexicon:
    """A Minimalist Grammar Lexicon"""

    def __init__(self, s: str) -> None: ...
    @staticmethod
    def random_lexicon(lemmas: list[str]) -> "Lexicon":
        """Generate a random lexicon from the list of lemmas"""

    def mdl(self, n_phonemes: int) -> float:
        """Returns the model description length of the lexicon"""

    def continuations(
        self,
        prefix: str,
        category: str,
        min_log_prob: float = -128.0,
        move_prob: float = 0.5,
        max_steps: int = 64,
        n_beams: int = 256,
        max_strings: Optional[int] = None,
    ) -> set[Continuation]:
        """Returns a set of all valid continuations from this prefix"""

    def generate_unique_strings(
        self,
        category: str,
        min_log_prob: float = -128.0,
        move_prob: float = 0.5,
        max_steps: int = 64,
        n_beams: int = 256,
        max_strings: Optional[int] = None,
    ) -> list[tuple[list[str], float]]:
        """Returns a list of all unique strings and their probabilities"""

    def generate_grammar(
        self,
        category: str,
        min_log_prob: float = -128.0,
        move_prob: float = 0.5,
        max_steps: int = 64,
        n_beams: int = 256,
        max_strings: Optional[int] = None,
    ) -> GrammarIterator:
        """Returns an iterator over all possible parses"""

    def parse(
        self,
        s: str,
        category: str,
        min_log_prob: float = -128.0,
        move_prob: float = 0.5,
        max_steps: int = 64,
        n_beams: int = 256,
        max_strings: Optional[int] = None,
    ) -> list[SyntacticStructure]:
        """Returns a list of all possible parses of that string.
        The string, s, should be delimited by spaces for words and hyphens for multi-word expressions from head-movement
        """

    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
