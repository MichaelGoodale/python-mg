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
///questions: list[str | Meaning]
///    Any questions to be asked in this scenario. (Must be LOT expressions)
///
///Raises
///------
///ValueError
///    If the questions are strings which are not proper LOT expressions.
#[pyclass(name = "Scenario", str, eq, from_py_object)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PyScenario {
    ///A list of Actors in the scenario
    #[pyo3(get, set)]
    actors: Vec<PyActor>,
    ///A list of Events in the scenario
    #[pyo3(get, set)]
    events: Vec<PyEvent>,

    ///A list of questions to be asked in the scenario
    #[pyo3(get)]
    questions: Vec<PyMeaning>,
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

        let questions = value
            .questions()
            .iter()
            .map(|x| {
                let s = x.to_string();
                PyMeaning::new(s).unwrap_or_else(|x| {
                    panic!("Internal library failure in parsing string:\n\t{x}",)
                })
            })
            .collect();

        PyScenario {
            actors,
            events,
            questions,
        }
    }
}

#[pymethods]
impl PyScenario {
    #[new]
    fn new(
        actors: Vec<PyActor>,
        events: Vec<PyEvent>,
        questions: Vec<MeaningOrString>,
    ) -> PyResult<Self> {
        Ok(Self {
            actors,
            events,
            questions: questions
                .into_iter()
                .map(MeaningOrString::into_meaning)
                .collect::<PyResult<_>>()?,
        })
    }

    #[setter]
    fn set_questions(&mut self, questions: Vec<MeaningOrString>) -> PyResult<()> {
        self.questions = questions
            .into_iter()
            .map(MeaningOrString::into_meaning)
            .collect::<Result<_, _>>()?;
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
