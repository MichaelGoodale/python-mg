.. Python Minimalist Grammars documentation master file, created by
   sphinx-quickstart on Wed Oct 15 11:30:51 2025.
   You can adapt this file completely to your liking, but it should at least
   contain the root `toctree` directive.

Python Minimalist Grammars documentation
========================================

This documents the `python-mg <github.com/MichaelGoodale/python-mg>`_ package which contains Python bindings for `a Minimalist Grammar parser written in Rust <https://github.com/MichaelGoodale/minimalist-grammar-parser>`_.
It provides the tools necessary to generate strings from a Minimalist Grammar and parse sentences in a Minimalist Grammar, as well as inspecting their parse tree.

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   lexicon
   syntax 
   metrics


Installation
------------

The package is made with `Maturin <https://github.com/PyO3/maturin>`_ and can be build however you'd like using that system.
The easiest way to build it or develop with it is by using `uv <https://github.com/astral-sh/uv>`_.

.. code-block:: bash

  git clone https://github.com/MichaelGoodale/python-mg
  cd python-mg
  uv run example.py

Otherwise, you can also install it with `pip` or other tools as a wheel by getting it from `the GitHub actions page <https://github.com/MichaelGoodale/python-mg/actions>`_

You can also add it to a uv project like so:

.. code-block:: bash 

  uv add git+https://github.com/MichaelGoodale/python-mg



Usage
-----

The following snippet declares a grammar, parses a sentence and generates a string in the grammar.

.. code-block:: python

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

The parse tree can also be turned into LaTeX code with Forest (see `latex-commands.tex <https://github.com/MichaelGoodale/python-mg/blob/master/latex-commands.tex>`_) or can be directly turned into a GraphViz Dot file.
