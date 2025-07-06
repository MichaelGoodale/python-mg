use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use anyhow::anyhow;
use logprob::LogProb;
use minimalist_grammar_parser::{
    Generator, ParsingConfig, PhonContent, RulePool, lexicon::Lexicon, parsing::beam::Continuation,
};
use pyo3::prelude::*;

mod graphing;
use graphing::{PyMgEdge, PyMgNode};

#[pyclass(name = "SyntacticStructure", str, eq, frozen)]
#[derive(Debug)]
struct PySyntacticStructure {
    prob: LogProb<f64>,
    string: Vec<PhonContent<String>>,
    rules: RulePool,
    lex: Py<PyLexicon>,
}

impl PartialEq for PySyntacticStructure {
    fn eq(&self, other: &Self) -> bool {
        self.prob == other.prob && self.string == other.string && self.rules == other.rules
    }
}

impl Display for PySyntacticStructure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.string.len();
        for (i, x) in self.string.iter().enumerate() {
            match x {
                PhonContent::Normal(s) => write!(f, "{s}")?,
                PhonContent::Affixed(items) => write!(f, "{}", items.join("-"))?,
            };
            if i != len - 1 {
                write!(f, " ")?;
            }
        }
        Ok(())
    }
}

#[pymethods]
impl PySyntacticStructure {
    fn log_prob(&self) -> f64 {
        self.prob.into_inner()
    }

    fn prob(&self) -> f64 {
        self.prob.into_inner().exp()
    }

    fn latex(&self) -> String {
        let lex = self.lex.get();
        self.rules.to_latex(&lex.0)
    }

    #[allow(clippy::type_complexity)]
    fn __to_tree_inner(&self) -> (Vec<(usize, PyMgNode)>, Vec<(usize, usize, PyMgEdge)>, usize) {
        let (g, root) = self.rules.to_tree(&self.lex.get().0);
        let nodes = g
            .node_indices()
            .map(|n| (n.index(), PyMgNode(g.node_weight(n).unwrap().clone())))
            .collect::<Vec<_>>();

        let edges = g
            .edge_indices()
            .map(|e| {
                let (src, tgt) = g.edge_endpoints(e).unwrap();
                (
                    src.index(),
                    tgt.index(),
                    PyMgEdge(*g.edge_weight(e).unwrap()),
                )
            })
            .collect::<Vec<_>>();

        (nodes, edges, root.index())
    }
}

#[pyclass(name = "Lexicon", str, eq, frozen)]
#[derive(Debug, Clone, Eq, PartialEq)]
struct PyLexicon(Lexicon<String, String>);

impl Display for PyLexicon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MGLexicon{{\n{}\n}}", self.0)
    }
}

#[pyclass]
struct GrammarIterator {
    generator: Generator<Lexicon<String, String>, String, String>,
    max_strings: Option<usize>,
    n_strings: usize,
    lexicon: Py<PyLexicon>,
}

#[pymethods]
impl GrammarIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PySyntacticStructure> {
        if let Some(n) = slf.max_strings {
            if slf.n_strings >= n {
                return None;
            }
        }

        if let Some((prob, string, rules)) = slf.generator.next() {
            slf.n_strings += 1;
            let py = slf.py();
            Some(PySyntacticStructure {
                prob,
                string,
                rules,
                lex: slf.lexicon.clone_ref(py),
            })
        } else {
            None
        }
    }
}

#[pyclass(name = "Continuation", str, eq, frozen, hash)]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct PyContinuation(Continuation<String>);

impl Display for PyContinuation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Continuation::Word(s) => write!(f, "{s}"),
            Continuation::AffixedWord(s) => write!(f, "{}", s.join("-")),
            Continuation::EndOfSentence => write!(f, "[EOS]"),
        }
    }
}

#[pymethods]
impl PyContinuation {
    #[new]
    fn new(s: &str) -> Self {
        match s {
            "[EOS]" => PyContinuation(Continuation::EndOfSentence),
            _ => PyContinuation(Continuation::Word(s.to_string())),
        }
    }

    fn __repr__(&self) -> String {
        format!("Continuation({})", self)
    }

    #[staticmethod]
    #[allow(non_snake_case)]
    fn EOS() -> PyContinuation {
        PyContinuation(Continuation::EndOfSentence)
    }

    fn is_end_of_string(&self) -> bool {
        matches!(self, PyContinuation(Continuation::EndOfSentence))
    }

    fn is_word(&self) -> bool {
        matches!(self, PyContinuation(Continuation::Word(_)))
    }
}

fn map_string(s: &str) -> Vec<PhonContent<String>> {
    match s.trim() {
        "" => vec![],
        _ => s
            .split(" ")
            .map(|x| {
                let x = x.split("-").map(|x| x.to_string()).collect::<Vec<_>>();
                if x.len() == 1 {
                    PhonContent::Normal(x.first().unwrap().clone())
                } else {
                    PhonContent::Affixed(x)
                }
            })
            .collect(),
    }
}

#[pymethods]
impl PyLexicon {
    fn mdl(&self, n_phonemes: u16) -> PyResult<f64> {
        Ok(self.0.mdl_score(n_phonemes).map_err(|e| anyhow!(e))?)
    }

    #[pyo3(signature = (prefix, category, min_log_prob=-128.0, move_prob=0.5, max_steps=64, n_beams=256))]
    fn continuations(
        &self,
        prefix: &str,
        category: String,
        min_log_prob: f64,
        move_prob: f64,
        max_steps: usize,
        n_beams: usize,
    ) -> PyResult<HashSet<PyContinuation>> {
        let config = ParsingConfig::new(
            LogProb::new(min_log_prob).map_err(|x| anyhow!(x.to_string()))?,
            LogProb::from_raw_prob(move_prob).map_err(|x| anyhow!(x.to_string()))?,
            max_steps,
            n_beams,
        );
        let prefix = map_string(prefix);

        Ok(self
            .0
            .valid_continuations(category, &prefix, &config)
            .map_err(|e| anyhow!(e.to_string()))?
            .into_iter()
            .map(PyContinuation)
            .collect())
    }

    #[staticmethod]
    fn random_lexicon(lemmas: Vec<String>) -> PyResult<Self> {
        let mut rng = rand::rng();
        let lex: Lexicon<_, u16> = Lexicon::random(&0, &lemmas, None, &mut rng);
        Ok(PyLexicon(
            lex.remap_lexicon(|x| x.clone(), |c| c.to_string()),
        ))
    }

    #[pyo3(signature = (category, min_log_prob=-128.0, move_prob=0.5, max_steps=64, n_beams=256, max_strings=None))]
    fn generate_unique_strings(
        &self,
        category: String,
        min_log_prob: f64,
        move_prob: f64,
        max_steps: usize,
        n_beams: usize,
        max_strings: Option<usize>,
    ) -> PyResult<Vec<(Vec<String>, f64)>> {
        let config = ParsingConfig::new(
            LogProb::new(min_log_prob).map_err(|x| anyhow!(x.to_string()))?,
            LogProb::from_raw_prob(move_prob).map_err(|x| anyhow!(x.to_string()))?,
            max_steps,
            n_beams,
        );

        let mut hashmap = HashMap::new();
        for (prob, string, _) in self.0.generate(category, &config).map_err(|e| anyhow!(e))? {
            hashmap
                .entry(string)
                .and_modify(|old_log_prob: &mut LogProb<f64>| {
                    *old_log_prob = old_log_prob.add_log_prob_clamped(prob)
                })
                .or_insert(prob);

            if let Some(max_strings) = max_strings {
                if hashmap.len() > max_strings {
                    break;
                }
            }
        }

        let mut values = hashmap.into_iter().collect::<Vec<_>>();
        values.sort_by_key(|x| x.1);
        Ok(values
            .into_iter()
            .map(|(s, p)| {
                (
                    s.into_iter()
                        .map(|x| match x {
                            PhonContent::Normal(s) => s,
                            PhonContent::Affixed(items) => items.join("-"),
                        })
                        .collect(),
                    p.into_inner(),
                )
            })
            .collect())
    }

    #[pyo3(signature = (category, min_log_prob=-128.0, move_prob=0.5, max_steps=64, n_beams=256, max_strings=None))]
    fn generate_grammar(
        slf: PyRef<'_, Self>,
        category: String,
        min_log_prob: f64,
        move_prob: f64,
        max_steps: usize,
        n_beams: usize,
        max_strings: Option<usize>,
    ) -> PyResult<GrammarIterator> {
        let config = ParsingConfig::new(
            LogProb::new(min_log_prob).map_err(|x| anyhow!(x.to_string()))?,
            LogProb::from_raw_prob(move_prob).map_err(|x| anyhow!(x.to_string()))?,
            max_steps,
            n_beams,
        );
        let py = slf.py();
        Ok(GrammarIterator {
            generator: slf
                .0
                .clone()
                .into_generate(category, &config)
                .map_err(|e| anyhow!(e))?,
            max_strings,
            lexicon: slf.into_pyobject(py).unwrap().into(),
            n_strings: 0,
        })
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (s, category, min_log_prob=-128.0, move_prob=0.5, max_steps=64, n_beams=256, max_parses=None))]
    fn parse(
        slf: PyRef<'_, Self>,
        s: &str,
        category: String,
        min_log_prob: f64,
        move_prob: f64,
        max_steps: usize,
        n_beams: usize,
        max_parses: Option<usize>,
    ) -> PyResult<Vec<PySyntacticStructure>> {
        let s = map_string(s);
        let config = ParsingConfig::new(
            LogProb::new(min_log_prob).map_err(|x| anyhow!(x.to_string()))?,
            LogProb::from_raw_prob(move_prob).map_err(|x| anyhow!(x.to_string()))?,
            max_steps,
            n_beams,
        );

        let parser = slf
            .0
            .parse(&s, category, &config)
            .map_err(|e| anyhow!(e.to_string()))?;

        let py = slf.py();
        let self_ref: Py<Self> = slf.clone().into_pyobject(py).unwrap().into();
        if let Some(max_parses) = max_parses {
            Ok(parser
                .take(max_parses)
                .map(|(prob, string, rules)| PySyntacticStructure {
                    prob,
                    rules,
                    string: string.to_vec(),
                    lex: self_ref.clone_ref(py),
                })
                .collect())
        } else {
            Ok(parser
                .map(|(prob, string, rules)| PySyntacticStructure {
                    prob,
                    rules,
                    string: string.to_vec(),
                    lex: self_ref.clone_ref(py),
                })
                .collect())
        }
    }

    #[new]
    fn new(s: &str) -> PyResult<PyLexicon> {
        Ok(PyLexicon(
            Lexicon::from_string(s).unwrap().to_owned_values(),
        ))
    }
}

/// A Python module implemented in Rust.
#[pymodule]
#[pyo3(name = "_lib_name")]
fn python_mg(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLexicon>()?;
    m.add_class::<PyContinuation>()?;
    m.add_class::<PySyntacticStructure>()?;
    m.add_class::<PyMgNode>()?;
    m.add_class::<PyMgEdge>()?;
    Ok(())
}
