use pyo3::prelude::*;
use simple_semantics::Event;

use super::*;

///Represents a type in the Language of Thought
///
/// The types are defined as follows:
///
/// * :math:`a`: Actors (things which receive theta-roles)
/// * :math:`e`: Events (things which assign theta-roles)
/// * :math:`t`: Truth values (true or false)
/// * :math:`\langle x, y \rangle`: a function from a type :math:`x` to a type :math:`y`
///
///
///Parameters
///----------
///t: str
///    The type of the string
#[pyclass(
    name = "LambdaType",
    module = "python_mg.semantics",
    eq,
    str,
    from_py_object,
    frozen
)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PyLambdaType(pub LambdaType);

impl Display for PyLambdaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[pymethods]
impl PyLambdaType {
    #[new]
    fn new(t: &str) -> PyResult<Self> {
        Ok(PyLambdaType(
            LambdaType::from_string(t).map_err(|e| PyValueError::new_err(e.to_string()))?,
        ))
    }

    ///Returns whether the type is a function type
    ///
    ///Returns
    ///-------
    ///bool
    ///    whether its a function
    fn is_function(&self) -> bool {
        self.0.is_function()
    }

    ///Returns the argument type of a function type
    ///
    ///Returns
    ///-------
    ///LambdaType
    ///    The left hand side of the type
    ///
    ///Raises
    ///------
    ///ValueError
    ///    If the type is not a function.
    fn lhs(&self) -> PyResult<PyLambdaType> {
        self.0
            .lhs()
            .map(|x| PyLambdaType(x.clone()))
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    ///Returns the result type of a function type
    ///
    ///Returns
    ///-------
    ///LambdaType
    ///    The right hand side of the type
    ///
    ///Raises
    ///------
    ///ValueError
    ///    If the type is not a function.
    fn rhs(&self) -> PyResult<PyLambdaType> {
        self.0
            .rhs()
            .map(|x| PyLambdaType(x.clone()))
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("LambdaType({self})")
    }

    fn __getnewargs__(&self) -> String {
        self.to_string()
    }
}

pub(super) fn convert_to_py_actor(name: &str, scenario: &Scenario<'_>) -> PyActor {
    PyActor {
        name: name.to_string(),
        properties: scenario
            .properties()
            .iter()
            .filter_map(|(prop, entries)| {
                if entries.contains(&Entity::Actor(name)) {
                    Some(prop.to_string())
                } else {
                    None
                }
            })
            .collect(),
    }
}

pub(super) fn convert_to_py_event(e_i: Event, scenario: &Scenario<'_>) -> Result<PyEvent, PyErr> {
    let e = scenario
        .thematic_relations()
        .get(e_i as usize)
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "Result is event {e_i}, but no such event exists in the scenario!"
            ))
        })?;

    Ok(PyEvent {
        agent: e.agent.map(|x| x.to_string()),
        patient: e.patient.map(|x| x.to_string()),
        properties: scenario
            .properties()
            .iter()
            .filter_map(|(prop, entries)| {
                if entries.contains(&Entity::Event(e_i)) {
                    Some(prop.to_string())
                } else {
                    None
                }
            })
            .collect(),
    })
}

///Represents an actor with a name and a set of properties to be used in Scenarios.
///
///Parameters
///----------
///name : str
///    The name of the actor.
///properties : set[str], optional
///    Any properties that apply to the actor. Defaults to an empty set.
///
///
///Examples
///--------
///Creating an actor and modifying its properties:
///
///.. code-block:: python
///
///    actor = Actor("John", properties={"mean", "unfriendly"})
///    actor.name = "Alice"
///    actor.properties = {"nice", "friendly"}
///
#[pyclass(
    name = "Actor",
    module = "python_mg.semantics",
    eq,
    str,
    from_py_object
)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PyActor {
    /// The name of the actor
    #[pyo3(get, set)]
    pub name: String,

    /// An unordered set of properties that apply to this actor
    #[pyo3(get, set)]
    pub properties: BTreeSet<String>,
}

#[pymethods]
impl PyActor {
    #[new]
    #[pyo3(signature = (name, properties=None))]
    fn new(name: String, properties: Option<BTreeSet<String>>) -> Self {
        PyActor {
            name,
            properties: properties.unwrap_or_default(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Actor({self})")
    }

    fn __getnewargs__(&self) -> (&str, &BTreeSet<String>) {
        (&self.name, &self.properties)
    }
}

impl Display for PyActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.name,
            if self.properties.is_empty() { "" } else { " (" },
            self.properties.iter().join(", "),
            if self.properties.is_empty() { "" } else { ")" },
        )
    }
}

///Represents an event to be used in a Scenario.
///
///Parameters
///----------
///agent : str, optional
///    The name of the agent (if there is one)
///patient : str, optional
///    The name of the patient (if there is one)
///properties : set[str], optional
///    Any properties that apply to the event. Defaults to an empty set.
///
///
///Examples
///--------
///Creating an event
///
///.. code-block:: python
///
///    running = Actor(agent="John", properties={"run", "quickly"})
///
#[pyclass(
    name = "Event",
    module = "python_mg.semantics",
    eq,
    str,
    from_py_object
)]
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct PyEvent {
    ///The agent of the event.
    #[pyo3(get, set)]
    pub agent: Option<String>,

    ///The patient of the event.
    #[pyo3(get, set)]
    pub patient: Option<String>,

    ///Any properties of the event.
    #[pyo3(get, set)]
    pub properties: BTreeSet<String>,
}

#[pymethods]
impl PyEvent {
    #[new]
    #[pyo3(signature = (agent=None, patient=None, properties=None))]

    fn new(
        agent: Option<String>,
        patient: Option<String>,
        properties: Option<BTreeSet<String>>,
    ) -> Self {
        PyEvent {
            agent,
            patient,
            properties: properties.unwrap_or_default(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Event({self})")
    }

    fn __getnewargs__(&self) -> (Option<&str>, Option<&str>, &BTreeSet<String>) {
        (
            self.agent.as_deref(),
            self.patient.as_deref(),
            &self.properties,
        )
    }
}

impl Display for PyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{{}{}{}{}{}{}}}",
            self.agent
                .as_deref()
                .map(|x| format!("A = {x}"))
                .unwrap_or("".to_string()),
            if self.patient.is_some() && self.agent.is_some() {
                ", "
            } else {
                ""
            },
            self.patient
                .as_deref()
                .map(|x| format!("P = {x}"))
                .unwrap_or("".to_string()),
            if self.properties.is_empty() { "" } else { " (" },
            self.properties.iter().join(" "),
            if self.properties.is_empty() { "" } else { ")" },
        )
    }
}

impl PyEvent {
    pub fn into_theta_roles<'a>(self: &'a PyEvent) -> ThetaRoles<'a> {
        ThetaRoles {
            agent: self.agent.as_deref(),
            patient: self.patient.as_deref(),
        }
    }
}
