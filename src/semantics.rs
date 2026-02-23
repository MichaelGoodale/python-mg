use std::{collections::BTreeMap, fmt::Display, sync::Arc};

use pyo3::{exceptions::PyValueError, prelude::*};
use simple_semantics::{Entity, EventType, PossibleEvent, Scenario, ScenarioIterator, ThetaRoles};

#[pyclass(name = "Scenario", str, eq, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PyScenario {
    actors: Vec<PyActor>,
    events: Vec<PyEvent>,
}

impl From<Scenario<'_>> for PyScenario {
    fn from(value: Scenario) -> Self {
        let actors = value
            .actors()
            .iter()
            .map(|x| {
                let properties = value
                    .properties()
                    .iter()
                    .filter_map(|(k, v)| {
                        if v.contains(&Entity::Actor(x)) {
                            Some(k.to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                PyActor {
                    name: x.to_string(),
                    properties,
                }
            })
            .collect();

        let events = value
            .thematic_relations()
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let properties = value
                    .properties()
                    .iter()
                    .filter_map(|(k, v)| {
                        if v.contains(&Entity::Event(u8::try_from(i).expect("Too many events!"))) {
                            Some(k.to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                PyEvent {
                    agent: x.agent.map(|x| x.to_string()),
                    patient: x.patient.map(|x| x.to_string()),
                    properties,
                }
            })
            .collect();

        PyScenario { actors, events }
    }
}

impl Display for PyScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_scenario())
    }
}

impl PyScenario {
    fn as_scenario<'a>(&'a self) -> Scenario<'a> {
        let actors = self.actors.iter().map(|x| x.name.as_str()).collect();
        let thematic_relations = self.events.iter().map(|x| x.into_theta_roles()).collect();
        let mut properties: BTreeMap<_, Vec<_>> = BTreeMap::new();

        for a in &self.actors {
            for p in &a.properties {
                properties
                    .entry(p.as_str())
                    .or_default()
                    .push(Entity::Actor(a.name.as_str()));
            }
        }
        for (i, e) in self.events.iter().enumerate() {
            for p in &e.properties {
                properties
                    .entry(p.as_str())
                    .or_default()
                    .push(Entity::Event(u8::try_from(i).expect("Too many events!")));
            }
        }

        Scenario::new(actors, thematic_relations, properties)
    }
}

#[pymethods]
impl PyScenario {
    #[new]
    fn new(s: String) -> PyResult<Self> {
        let scenario =
            Scenario::parse(s.as_str()).map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(scenario.into())
    }

    fn __repr__(&self) -> String {
        format!("Scenario({self})")
    }

    #[staticmethod]
    fn all_scenarios(
        actors: Vec<String>,
        event_kinds: Vec<PyPossibleEvent>,
        actor_properties: Vec<String>,
    ) -> PyScenarioIterator {
        let parameter_holder = Arc::new(ParameterHolder {
            actors,
            event_kinds,
            actor_properties,
        });

        let actors: Vec<&'static str> = parameter_holder
            .actors
            .iter()
            .map(|x| {
                let s: &'static str = unsafe { std::mem::transmute(x.as_str()) };
                s
            })
            .collect::<Vec<_>>();
        let properties: Vec<&'static str> = parameter_holder
            .actor_properties
            .iter()
            .map(|x| {
                let s: &'static str = unsafe { std::mem::transmute(x.as_str()) };
                s
            })
            .collect::<Vec<_>>();

        let event_kinds: Vec<PossibleEvent<'static>> = parameter_holder
            .event_kinds
            .iter()
            .map(|x| {
                let x = x.as_possible_event();
                let x: PossibleEvent<'static> = unsafe { std::mem::transmute(x) };
                x
            })
            .collect::<Vec<_>>();

        PyScenarioIterator {
            generator: Scenario::all_scenarios(&actors, &event_kinds, &properties),
            _parameter_holder: parameter_holder,
        }
    }
}

#[pyclass(name = "PossibleEvent", eq, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PyPossibleEvent {
    #[pyo3(get, set)]
    pub has_agent: bool,
    #[pyo3(get, set)]
    pub has_patient: bool,
    pub is_reflexive: bool,
    #[pyo3(get, set)]
    pub name: String,
}

impl PyPossibleEvent {
    fn event_type(&self) -> EventType {
        match (self.has_agent, self.has_patient) {
            (true, true) if self.is_reflexive => EventType::Transitive,
            (true, true) => EventType::TransitiveNonReflexive,
            (true, false) => EventType::Unergative,
            (false, true) => EventType::Unaccusative,
            (false, false) => EventType::Avalent,
        }
    }

    fn as_possible_event<'a>(&'a self) -> PossibleEvent<'a> {
        PossibleEvent {
            label: self.name.as_str(),
            event_type: self.event_type(),
        }
    }
}

#[pyclass(name = "Actor", eq, str, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PyActor {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub properties: Vec<String>,
}

impl Display for PyActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}",
            self.name,
            if self.properties.is_empty() { "" } else { " (" },
            self.properties.join(", "),
            if self.properties.is_empty() { "" } else { ")" },
        )
    }
}

#[pyclass(name = "Event", eq, str, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PyEvent {
    #[pyo3(get, set)]
    pub agent: Option<String>,
    #[pyo3(get, set)]
    pub patient: Option<String>,
    #[pyo3(get, set)]
    pub properties: Vec<String>,
}
impl Display for PyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{{}{}{}}}",
            self.agent.as_deref().unwrap_or(""),
            if self.patient.is_some() && self.agent.is_some() {
                " "
            } else {
                ""
            },
            self.patient.as_deref().unwrap_or("")
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

#[pyclass(name = "ScenarioGenerator")]
pub struct PyScenarioIterator {
    generator: ScenarioIterator<'static>,
    _parameter_holder: Arc<ParameterHolder>,
}

#[pymethods]
impl PyScenarioIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyScenario> {
        slf.generator.next().map(|s| s.into())
    }
}

struct ParameterHolder {
    actors: Vec<String>,
    event_kinds: Vec<PyPossibleEvent>,
    actor_properties: Vec<String>,
}
