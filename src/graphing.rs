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
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
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

#[pyclass(name = "MGEdge", str, eq, frozen)]
#[derive(Debug, Clone, PartialEq, Eq)]
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
