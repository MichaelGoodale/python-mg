use minimalist_grammar_parser::{
    parsing::rules::{MGEdge, MgNode},
    Direction,
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
                match lemma {
                    Some(x) => x,
                    None => "ε",
                },
                features
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            MgNode::Trace(trace_id) => write!(f, "t{trace_id}"),
        }
    }
}

#[pymethods]
impl PyMgNode {
    fn is_trace(&self) -> bool {
        matches!(self.0, MgNode::Trace(_))
    }

    fn trace_id(&self) -> PyResult<usize> {
        match &self.0 {
            MgNode::Node { .. } | MgNode::Leaf { .. } => Err(anyhow::anyhow!("Not a trace node!"))?,
            MgNode::Trace(trace_id) => Ok(trace_id.index()),
        }
    }

    fn lemma_string(&self) -> String {
        match &self.0 {
            MgNode::Node { .. } => "".to_string(),
            MgNode::Leaf { lemma, .. } => match lemma {
                Some(s) => s.clone(),
                None => "".to_string(),
            },
            MgNode::Trace(trace_id) => format!("t{trace_id}"),
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
}
