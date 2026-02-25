use super::*;

///Represents a Scenario, a model that meanings are evaluated in.
///
///Parameters
///----------
///actors : list[Actor]
///    The actors present in the scenario
///events: list[Event]
///    The events happening in the scenario
///events: list[str]
///    The questions in a scenario. (Will raise a `ValueError` if set with a `str` which is not a
///    valid Language of Thought expression)
#[pyclass(name = "Scenario", str, eq, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PyScenario {
    #[pyo3(get, set)]
    actors: Vec<PyActor>,
    #[pyo3(get, set)]
    events: Vec<PyEvent>,

    #[pyo3(get)]
    questions: Vec<String>,
}

impl From<Scenario<'_>> for PyScenario {
    fn from(value: Scenario) -> Self {
        let actors = value
            .actors()
            .iter()
            .map(|x| PyActor {
                name: x.to_string(),
                properties: value
                    .properties()
                    .iter()
                    .filter_map(|(k, v)| {
                        if v.contains(&Entity::Actor(x)) {
                            Some(k.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
            })
            .collect();

        let events = value
            .thematic_relations()
            .iter()
            .enumerate()
            .map(|(i, x)| PyEvent {
                agent: x.agent.map(|x| x.to_string()),
                patient: x.patient.map(|x| x.to_string()),
                properties: value
                    .properties()
                    .iter()
                    .filter_map(|(k, v)| {
                        if v.contains(&Entity::Event(u8::try_from(i).expect("Too many events!"))) {
                            Some(k.to_string())
                        } else {
                            None
                        }
                    })
                    .collect(),
            })
            .collect();

        let questions = value.questions().iter().map(|x| x.to_string()).collect();

        PyScenario {
            actors,
            events,
            questions,
        }
    }
}

#[pymethods]
impl PyScenario {
    #[setter]
    fn set_questions(&mut self, questions: Vec<String>) -> PyResult<()> {
        for q in &questions {
            let _ = RootedLambdaPool::parse(q).map_err(|e| PyValueError::new_err(e.to_string()))?;
        }

        self.questions = questions;
        Ok(())
    }
}

impl Display for PyScenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_scenario())
    }
}

impl PyScenario {
    pub(super) fn as_scenario<'a>(&'a self) -> Scenario<'a> {
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
