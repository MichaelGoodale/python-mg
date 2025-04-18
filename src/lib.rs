use std::fmt::Display;

use anyhow::anyhow;
use logprob::LogProb;
use minimalist_grammar_parser::{lexicon::Lexicon, Generator, ParsingConfig, RulePool};
use pyo3::prelude::*;

mod graphing;
use graphing::{PyMgEdge, PyMgNode};

#[pyclass(name = "SyntacticStructure", str, eq, frozen)]
#[derive(Debug)]
struct PySyntacticStructure {
    prob: LogProb<f64>,
    string: Vec<String>,
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
        write!(f, "{}", self.string.join(" "))
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
    fn __to_tree_inner(&self) -> (Vec<(usize, PyMgNode)>, Vec<(usize, usize, PyMgEdge)>) {
        let (g, _root) = self.rules.to_tree(&self.lex.get().0);
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

        (nodes, edges)
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
struct GrammarIterator(
    Generator<Lexicon<String, String>, String, String>,
    Py<PyLexicon>,
);

#[pymethods]
impl GrammarIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PySyntacticStructure> {
        let py = slf.py();
        slf.0
            .next()
            .map(|(prob, string, rules)| PySyntacticStructure {
                prob,
                string,
                rules,
                lex: slf.1.clone_ref(py),
            })
    }
}

#[pymethods]
impl PyLexicon {
    #[pyo3(signature = (category, min_log_prob=-128.0, move_prob=0.5, max_steps=64, n_beams=256))]
    fn generate_grammar(
        slf: PyRef<'_, Self>,
        category: String,
        min_log_prob: f64,
        move_prob: f64,
        max_steps: usize,
        n_beams: usize,
    ) -> PyResult<GrammarIterator> {
        let config = ParsingConfig::new(
            LogProb::new(min_log_prob).map_err(|x| anyhow!(x.to_string()))?,
            LogProb::from_raw_prob(move_prob).map_err(|x| anyhow!(x.to_string()))?,
            max_steps,
            n_beams,
        );
        let py = slf.py();
        Ok(GrammarIterator(
            Generator::new(slf.0.clone(), category, &config)?,
            slf.into_pyobject(py).unwrap().into(),
        ))
    }

    #[new]
    fn new(s: &str) -> PyResult<PyLexicon> {
        Ok(PyLexicon(Lexicon::parse(s).unwrap().to_owned_values()))
    }
}

/// A Python module implemented in Rust.
#[pymodule]
#[pyo3(name = "_lib_name")]
fn python_mg(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyLexicon>()?;
    m.add_class::<PySyntacticStructure>()?;
    m.add_class::<PyMgNode>()?;
    m.add_class::<PyMgEdge>()?;
    Ok(())
}
