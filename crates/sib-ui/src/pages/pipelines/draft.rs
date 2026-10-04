use sib_core::pipeline::slug;
use sib_core::{Pipeline, Step, Variable};

#[derive(Clone)]
pub struct Draft {
    pub pipeline: Pipeline,
    pub original_id: Option<String>,
    pub id_is_manual: bool,
    pub error: Option<String>,
}

impl Draft {
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline {
                steps: vec![Step::default()],
                ..Default::default()
            },
            original_id: None,
            id_is_manual: false,
            error: None,
        }
    }

    pub fn edit(pipeline: &Pipeline) -> Self {
        Self {
            pipeline: pipeline.clone(),
            original_id: Some(pipeline.id.clone()),
            id_is_manual: true,
            error: None,
        }
    }

    pub fn duplicate(pipeline: &Pipeline) -> Self {
        let mut copy = pipeline.clone();
        copy.id = format!("{}-copy", pipeline.id);
        copy.name = format!("{} (copy)", pipeline.name);
        Self {
            pipeline: copy,
            original_id: None,
            id_is_manual: true,
            error: None,
        }
    }

    pub fn sync_id(&mut self) {
        if !self.id_is_manual {
            self.pipeline.id = slug(&self.pipeline.name);
        }
    }

    pub fn add_variable(&mut self) {
        self.pipeline.variables.push(Variable::default());
    }

    pub fn add_step(&mut self) {
        self.pipeline.steps.push(Step::default());
    }

    pub fn move_step(&mut self, index: usize, delta: isize) {
        let steps = &mut self.pipeline.steps;
        let target = index as isize + delta;
        if target < 0 || target as usize >= steps.len() {
            return;
        }
        steps.swap(index, target as usize);
    }

    pub fn is_rename(&self) -> bool {
        self.original_id
            .as_ref()
            .is_some_and(|original| original != &self.pipeline.id)
    }
}
