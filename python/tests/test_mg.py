import pytest

from python_mg import Lexicon


def test_lexicon():
    x = Lexicon("a::b= a\nb::b")
    assert [str(s) for s in x.generate_grammar("a")] == ["a b"]
    parse = next(x.generate_grammar("a"))
    assert (
        parse.latex()
        == "\\begin{forest}\n[{\\der{a}}\n\t[{\\plainlex{a}{\\cancel{b=} a}} ]\n\t[{\\plainlex{b}{\\cancel{b}}} ] ]\n\\end{forest}"
    )
