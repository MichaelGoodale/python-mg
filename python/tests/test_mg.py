import pytest

from python_mg import Lexicon, Continuation


def test_lexicon():
    x = Lexicon("a::b= a\nb::b")
    assert [str(s) for s in x.generate_grammar("a")] == ["a b"]
    parse = next(x.generate_grammar("a"))
    assert (
        parse.latex()
        == "\\begin{forest}[\\der{a} [\\plainlex{b= a}{a}] [\\plainlex{b}{b}]]\\end{forest}"
    )


def test_continuations():
    x = Lexicon("a::b= S\nb::b")
    assert x.continuations("a", "S") == {Continuation("b")}
    x = Lexicon("a::S= b= S\n::S\nb::b")
    assert x.continuations("", "S") == {Continuation("[EOS]"), Continuation("a")}
    assert x.continuations("a", "S") == {Continuation("b"), Continuation("a")}
    assert x.continuations("a b", "S") == {Continuation("[EOS]")}
    assert x.continuations("a b", "S") == {Continuation.EOS()}
    assert x.continuations("a a", "S") == {Continuation("b"), Continuation("a")}
    assert x.continuations("a a b", "S") == {Continuation("b")}
