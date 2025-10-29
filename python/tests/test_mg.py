import pytest
import pickle

from python_mg import Lexicon, Continuation
from python_mg.syntax import Trace, Mover


def test_lexicon():
    x = Lexicon("a::b= a\nb::b")
    assert [str(s) for s in x.generate_grammar("a")] == ["a b"]
    parse = next(x.generate_grammar("a"))
    assert (
        parse.latex()
        == "\\begin{forest}[\\der{a} [\\plainlex{b= a}{a}] [\\plainlex{b}{b}]]\\end{forest}"
    )


def test_pickling():
    x = Lexicon("a::b= a\nb::b")
    x_pickle = pickle.dumps(x)
    assert pickle.loads(x_pickle) == x


def test_memory_load():
    grammar = Lexicon("a::b= c= +a +e C\nb::b -a\nc::c -e")
    parse = grammar.parse("c b a", "C")[0]
    assert parse.max_memory_load() == 2
    grammar = Lexicon("a::b= +a c= +e C\nb::b -a\nc::c -e")
    parse = grammar.parse("c b a", "C")[0]
    assert parse.max_memory_load() == 1


def test_trees():
    grammar = """
::T= C
::T= +W C
s::=>V =D T
know::C= V
say::C= V
prefer::D= V
drink::D= V
king::N
wine::N
beer::N
queen::N
the::N= D
which::N= D -W"""
    lexicon = Lexicon(grammar)

    for p in lexicon.parse("which beer the queen drink-s", "C"):
        tree = p.to_tree()
        assert (
            p.latex()
            == "\\begin{forest}[\\der{C} [\\der{D -W} [\\plainlex{N= D -W}{which}] [\\plainlex{N}{beer}]] [\\der{+W C} [\\plainlex{T= +W C}{$\\epsilon$}] [\\der{T} [\\der{D} [\\plainlex{N= D}{the}] [\\plainlex{N}{queen}]] [\\der{=D T} [\\plainlex{=>V =D T}{drink-s}] [\\der{V} [\\plainlex{D= V}{drink}] [$t_0$]]]]]]\\end{forest}"
        )
        assert tree.normal_string() == "which beer the queen drink-s"

        # A rich string which illustrates where movement was generated from
        base = tree.base_string()

        print(base)
        assert base[-1] == Trace(0)

        assert tree.base_string() == [
            Mover(s=["which", "beer"], trace=0),
            "ε",
            "the",
            "queen",
            "drink-s",
            Trace(trace=0),
        ]

        digraph = """digraph {
0 [label="C", ordering=out];
1 [label="D -W", ordering=out];
2 [label="+W C", ordering=out];
3 [label="which::N= D -W", ordering=out];
4 [label="beer::N", ordering=out];
5 [label="ε::T= +W C", ordering=out];
6 [label="T", ordering=out];
7 [label="D", ordering=out];
8 [label="=D T", ordering=out];
9 [label="the::N= D", ordering=out];
10 [label="queen::N", ordering=out];
11 [label="drink-s::=>V =D T", ordering=out];
12 [label="V", ordering=out];
13 [color=gray, fontcolor=gray, label="drink::D= V", ordering=out, style=dashed];
14 [label="t0", ordering=out];
0 -> 1 ;
1 -> 3 ;
2 -> 5 ;
6 -> 7 ;
7 -> 9 ;
8 -> 11 ;
12 -> 13 ;
0 -> 2 ;
1 -> 4 ;
2 -> 6 ;
6 -> 8 ;
7 -> 10 ;
8 -> 12 ;
12 -> 14 ;
14 -> 1 [constraint=false, style=dashed];
13 -> 11 [constraint=false, style=dashed];
}
"""
        assert tree.to_dot() == digraph


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

    parses = x.parse("a a b b", "S")
    for parse in parses:
        assert parse.contains_word("a")
        assert parse.contains_word("b")
        assert parse.contains_word("")
        assert parse.contains_word(None)
        assert not parse.contains_word("c")

    x = Lexicon("a::B= S\na::S\nb::B")
    for parse in x.parse("a", "S"):
        assert parse.contains_word("a")
        assert not parse.contains_word("b")
        assert not parse.contains_word("")
        assert not parse.contains_word(None)
        assert not parse.contains_word("c")
        assert not parse.contains_lexical_entry("a::B= S")
        assert not parse.contains_lexical_entry("b::B")
        assert parse.contains_lexical_entry("a::S")

    for parse in x.parse("a b", "S"):
        assert parse.contains_word("a")
        assert parse.contains_word("b")
        assert not parse.contains_word("")
        assert not parse.contains_word(None)
        assert not parse.contains_word("c")
        assert parse.contains_lexical_entry("a::B= S")
        assert parse.contains_lexical_entry("b::B")
        assert not parse.contains_lexical_entry("a::S")

    lexicon = Lexicon(
        """::T<= +q Q
what::d[in] -subj3 -q -wh
what::d[in] -acc -wh
who::d[an] -subj3 -q -wh
who::d[an] -acc -wh
::T<= +q +wh Q
::q -q
does::V= q= +subj3 T
do::V= q= +subj2 T
do::V= q= +subj1 T
did::V= q= +subj3 T
did::V= q= +subj2 T
did::V= q= +subj1 T
::q -q
to::theme[an]= p
talk::p= v
see::d[an]= +acc v
see::d[in]= +acc v
devour::d[in]= +acc v
want::d[in]= +acc v
run::v
you::d[an] -subj2
you::d[an] -acc
I::d[an] -subj1
me::d[an] -acc
he::d[an] -subj3
him::d[an] -acc
she::d[an] -subj3
her::d[an] -acc
::d[an]= +theme theme[an]
that::C= +r +rel[in] d[in] -acc
that::C= +r +rel[in] d[in] -subj3
who::C= +r +rel[an] d[an] -acc
who::C= +r +rel[an] d[an] -subj3
::=>v =d[an] V
man::N[an]
woman::N[an]
cake::N[in]
John::d[an] -subj3
John::d[an] -acc
Mary::d[an] -subj3
Mary::d[an] -acc
the::N[in]= d[in] -theme
the::N[in]= d[in] -subj3
the::N[in]= d[in] -acc
the[OBJ_REL]::N[in]= d[in] -acc -rel[in]
the[SUB_REL]::N[in]= d[in] -subj3 -rel[in]
the::N[an]= d[an] -theme
the::N[an]= d[an] -subj3
the::N[an]= d[an] -acc
the[OBJ_REL]::N[an]= d[an] -acc -rel[an]
the[SUB_REL]::N[an]= d[an] -subj3 -rel[an]
a::N[in]= d[in] -theme
a::N[in]= d[in] -subj3
a::N[in]= d[in] -acc
a[OBJ_REL]::N[in]= d[in] -acc -rel[in]
a[SUB_REL]::N[in]= d[in] -subj3 -rel[in]
a::N[an]= d[an] -theme
a::N[an]= d[an] -subj3
a::N[an]= d[an] -acc
a[OBJ_REL]::N[an]= d[an] -acc -rel[an]
a[SUB_REL]::N[an]= d[an] -subj3 -rel[an]
can::V= +subj3 T
can::V= +subj2 T
can::V= +subj1 T
can::V= q= +subj3 T
can::V= q= +subj2 T
can::V= q= +subj1 T
can::V= r= +subj3 T
can::V= r= +subj2 T
can::V= r= +subj1 T
am::prog= +subj1 T
are::prog= +subj2 T
is::prog= +subj3 T
am::prog= q= +subj1 T
are::prog= q= +subj2 T
is::prog= q= +subj3 T
am::prog= r= +subj1 T
are::prog= r= +subj2 T
is::prog= r= +subj3 T
ing::=>V prog
PAST::=>V +subj3 t
PAST::=>V +subj2 t
PAST::=>V +subj1 t
::T= C
::t= T
::t= r= T
::r -r
3PRES::=>V +subj3 t
2PRES::=>V +subj2 t
1PRES::=>V +subj1 t"""
    )

    assert lexicon.continuations("he is run-ing", "C") == {Continuation.EOS()}
