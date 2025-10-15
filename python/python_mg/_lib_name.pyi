from typing import Sequence
import numpy as np
import numpy.typing as npt

from python_mg.syntax import ParseTree

class MGNode:
    def is_trace(self) -> bool: ...
    def trace_id(self) -> int:
        """Gets the trace id of traces and raises an error otherwise"""

    def lemma_string(self) -> str:
        """Format the node as a string in a tree if leaf or trace"""

class MGEdge:
    def is_move(self) -> bool:
        """Checks whether the edge is a movement edge"""

    @staticmethod
    def move_edge() -> MGEdge:
        """Gets a movement edge"""

class SyntacticStructure:
    """A parse tree for some string"""

    def __init__(self) -> None: ...
    def log_prob(self) -> float:
        """Return the log probability."""

    def n_steps(self) -> int:
        """Returns the number of steps in the derivation of this structure"""

    def contains_lexical_entry(self, s: str) -> bool:
        """Check if this structure contains a specific lexical entry (formatted as an MG entry, will raise an error if unparseable)"""

    def contains_word(self, s: str | None) -> bool:
        """Check if this structure contains a specific word."""

    def prob(self) -> float:
        """Return the probability of this syntactic structure."""

    def latex(self) -> str:
        """Return a LaTeX representation of this syntactic structure."""

    def to_tree(self) -> ParseTree:
        """Converts a syntactic structure into a graph structure"""

    def max_memory_load(self) -> int:
        """Gets the largest amount of movers at a single point"""

    def tokens(self) -> npt.NDArray[np.uint]:
        """Converts the string of this SyntacticStructure into a tokenized numpy array"""

    def __to_tree_inner(
        self,
    ) -> tuple[list[tuple[int, MGNode]], list[tuple[int, int, MGEdge]], int]: ...

class Continuation:
    """A continuation of a prefix string"""

    def __init__(self, word: str) -> None: ...
    @staticmethod
    def EOS() -> "Continuation": ...
    def is_end_of_string(self) -> bool:
        """Check if the continuation is a end of string marker"""

    def is_word(self) -> bool:
        """Check if the continuation is a word"""

    def is_multi_word(self) -> bool:
        """Check if the continuation is an affixed word"""

class GrammarIterator:
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
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> set[Continuation]:
        """Returns a set of all valid continuations from this prefix"""

    def generate_unique_strings(
        self,
        category: str,
        min_log_prob: float = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> list[tuple[list[str], float]]:
        """Returns a list of all unique strings and their probabilities"""

    def generate_grammar(
        self,
        category: str,
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> GrammarIterator:
        """Returns an iterator over all possible parses"""

    def parse(
        self,
        s: str,
        category: str,
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> list[SyntacticStructure]:
        """Returns a list of all possible parses of that string.
        The string, s, should be delimited by spaces for words and hyphens for multi-word expressions from head-movement
        """

    def parse_tokens(
        self,
        s: Sequence[int] | npt.NDArray[np.uint],
        category: str,
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
        max_strings: int | None = None,
    ) -> list[SyntacticStructure]:
        """Returns a list of all possible parses of a string represented by tokens."""

    def tokens(self) -> dict[str, int]:
        """Returns the mapping from strings to token IDS"""

    def detokenize(self, s: Sequence[int] | npt.NDArray[np.uint]) -> list[str]:
        """Takes a sequence of tokens and converts them to their strings"""

    def detokenize_batch(
        self,
        s: Sequence[Sequence[int]] | list[npt.NDArray[np.uint]] | npt.NDArray[np.uint],
    ) -> list[list[str]]:
        """Takes a sequence of tokens and converts them to their strings"""

    def token_continuations(
        self,
        x: npt.NDArray[np.uint],
        category: str,
        min_log_prob: float | None = -128.0,
        move_prob: float = 0.5,
        max_steps: int | None = 64,
        n_beams: int | None = 256,
    ) -> npt.NDArray[np.bool]:
        """Takes a [.., N, L] numpy array of strings in a grammar and returns the valid next move for each index as a [..., N, L, C] boolean array.
        L corresponds to length of maximum strings, N to the number of strings and C to the number of words in the grammar.
        """
