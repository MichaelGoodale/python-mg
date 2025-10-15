# Python Minimalist Grammar Parser

This repository contains Python bindings for [a Minimalist Grammar parser written in Rust](https://github.com/MichaelGoodale/minimalist-grammar-parser).
It provides the tools necessary to generate strings from a Minimalist Grammar and parse sentences in a Minimalist Grammar, as well as inspecting their parse tree.

## Installation

The package is made with [Maturin](https://github.com/PyO3/maturin) and can be build however you'd like using that system.
The easiest way to build it or develop with it is by using [uv](https://github.com/astral-sh/uv).

```bash
git clone https://github.com/MichaelGoodale/python-mg
cd python-mg
uv run example.py
```

Otherwise, you can also install it with `pip` or other tools as a wheel by getting it from [the GitHub actions page](https://github.com/MichaelGoodale/python-mg/actions)

You can also add it to a uv project like so:

```bash
uv add git+https://github.com/MichaelGoodale/python-mg
```

## Usage

The following snippet declares a grammar, parses a sentence and generates a string in the grammar.

```python
from python_mg import Lexicon

grammar = """
::V= C
::V= +W C
knows::C= =D V
says::C= =D V
prefers::D= =D V
drinks::D= =D V
king::N
wine::N
beer::N
queen::N
the::N= D
which::N= D -W
"""
lexicon = Lexicon(grammar)

for p in lexicon.parse("which beer the queen drinks", "C"):
    tree = p.to_tree()
    tree.to_image().show()

for p in lexicon.generate_grammar("C", max_strings=1):
    print(p)
    print(p.latex())
    print(p.to_tree().to_dot())
    p.to_tree().to_image().show()
```

The parse tree can also be turned into LaTeX code with Forest (see [`latex-commands.tex`](https://github.com/MichaelGoodale/python-mg/blob/master/latex-commands.tex)) or can be directly turned into a GraphViz Dot file.

## Examples

There are some examples of how to apply the code in the [examples directory](./examples/)

These include:

- [A script to generate strings from an MG](examples/generate_strings.py)
- [A script for training a transformer on an MG](examples/train_llm_from_mg.py)

Some of the scripts require extra dependencies.
You can test them with the following command in `uv`

```bash
uv run --group examples examples/train_llm_from_mg.py
```
