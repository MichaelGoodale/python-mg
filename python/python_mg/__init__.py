from python_mg._lib_name import Lexicon, Continuation, SyntacticStructure
from python_mg.syntax import to_tree

SyntacticStructure.to_tree = to_tree
__all__ = [
    "Lexicon",
    "Continuation",
    "SyntacticStructure",
]
