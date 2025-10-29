use minimalist_grammar_parser::parsing::rules::{TreeEdge, TreeNode};
use pyo3::{exceptions::PyValueError, prelude::*};
use std::fmt::Display;

#[pyclass(name = "MGNode", str, eq, frozen)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyMgNode(pub TreeNode<'static, String, String>);

impl Display for PyMgNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[pymethods]
impl PyMgNode {
    fn is_trace(&self) -> bool {
        self.0.is_trace()
    }

    fn trace_id(&self) -> PyResult<usize> {
        self.0
            .trace_id()
            .map(|x| x.into())
            .ok_or(PyValueError::new_err("Not a trace!"))
    }

    fn lemma_string(&self) -> String {
        self.0.lemma().map(|x| x.to_string()).unwrap_or_default()
    }

    fn is_stolen(&self) -> bool {
        self.0.lemma().map(|x| x.is_stolen()).unwrap_or(false)
    }
}

#[pyclass(name = "MGEdge", str, eq, frozen)]
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct PyMgEdge(pub TreeEdge);

impl Display for PyMgEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.0 {
                TreeEdge::Merge(minimalist_grammar_parser::Direction::Left) => "MergeLeft",
                TreeEdge::Merge(minimalist_grammar_parser::Direction::Right) => "MergeRight",
                TreeEdge::Move => "Move",
                TreeEdge::MoveHead => "MoveHead",
            }
        )
    }
}

#[pymethods]
impl PyMgEdge {
    fn is_move(&self) -> bool {
        matches!(self.0, TreeEdge::Move)
    }

    fn is_head_move(&self) -> bool {
        matches!(self.0, TreeEdge::MoveHead)
    }

    fn is_merge(&self) -> bool {
        matches!(self.0, TreeEdge::Merge(_))
    }

    fn __repr__(&self) -> String {
        let s = match self.0 {
            TreeEdge::Merge(minimalist_grammar_parser::Direction::Left) => "MergeLeft",
            TreeEdge::Merge(minimalist_grammar_parser::Direction::Right) => "MergeRight",
            TreeEdge::Move => "Move",
            TreeEdge::MoveHead => "MoveHead",
        };
        format!("MGEdge({s})")
    }
}
