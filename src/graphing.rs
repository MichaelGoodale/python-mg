use minimalist_grammar_parser::{
    Direction,
    parsing::rules::{MGEdge, MgNode},
};
use pyo3::prelude::*;
use std::fmt::Display;

#[pyclass(name = "MGNode", str, eq, frozen)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyMgNode(pub MgNode<String, String>);

impl Display for PyMgNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            MgNode::Node {
                features, movement, ..
            } if movement.is_empty() => write!(
                f,
                "{}",
                features
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            MgNode::Node {
                features, movement, ..
            } => write!(
                f,
                "{} {{{}}}",
                features
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                movement
                    .iter()
                    .map(|x| x
                        .features()
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(" "))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            MgNode::Leaf {
                lemma, features, ..
            } => write!(
                f,
                "{}::{}",
                lemma.to_string("ε", "-"),
                features
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            MgNode::Trace { trace, .. } => write!(f, "t{trace}"),
        }
    }
}

#[pymethods]
impl PyMgNode {
    fn is_trace(&self) -> bool {
        matches!(self.0, MgNode::Trace { .. })
    }

    fn trace_id(&self) -> PyResult<usize> {
        match &self.0 {
            MgNode::Node { .. } | MgNode::Leaf { .. } => Err(anyhow::anyhow!("Not a trace node!"))?,
            MgNode::Trace { trace, .. } => Ok(trace.index()),
        }
    }

    fn lemma_string(&self) -> String {
        match &self.0 {
            MgNode::Node { .. } => "".to_string(),
            MgNode::Leaf { lemma, .. } => lemma.to_string("ε", "-"),
            MgNode::Trace { trace, .. } => format!("t{trace}"),
        }
    }
}

#[pyclass(name = "MGEdge", str, eq, frozen)]
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct PyMgEdge(pub MGEdge);

impl Display for PyMgEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            MGEdge::Move => write!(f, "move"),
            MGEdge::Merge(d) => write!(
                f,
                "{}",
                match d {
                    Some(Direction::Left) => "L",
                    Some(Direction::Right) => "R",
                    None => "",
                }
            ),
        }
    }
}

#[pymethods]
impl PyMgEdge {
    fn is_move(&self) -> bool {
        matches!(self.0, MGEdge::Move)
    }

    #[staticmethod]
    fn move_edge() -> PyMgEdge {
        PyMgEdge(MGEdge::Move)
    }
}
