use super::*;


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

pub(super) fn convert_to_py_event(e_i: u8, scenario: &Scenario<'_>) -> Result<PyEvent, PyErr> {
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


#[pyclass(name = "Actor", eq, str, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PyActor {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub properties: BTreeSet<String>,
}

#[pymethods]
impl PyActor {
    #[new]
    #[pyo3(signature = (name, properties=None))]
    ///Parameters
    ///----------
    ///name : str
    ///    The name of the actor.
    ///properties: set[str], optional
    ///    Any properties that apply to the actor.
    ///Returns
    ///-------
    ///Actor
    fn new(name: String, properties: Option<BTreeSet<String>>) -> Self {
        PyActor {
            name,
            properties: properties.unwrap_or_default(),
        }
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


#[pyclass(name = "Event", eq, str, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct PyEvent {
    #[pyo3(get, set)]
    pub agent: Option<String>,
    #[pyo3(get, set)]
    pub patient: Option<String>,
    #[pyo3(get, set)]
    pub properties: BTreeSet<String>,
}

#[pymethods]
impl PyEvent {
    #[new]
    #[pyo3(signature = (agent=None, patient=None, properties=None))]
    ///Parameters
    ///----------
    ///agent : str, optional
    ///    The name of the agent (if there is one)
    ///patient : str, optional
    ///    The name of the patient (if there is one)
    ///properties: set[str], optional
    ///    Any properties that apply to the actor.
    ///Returns
    ///-------
    ///Event
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
