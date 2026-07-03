#![allow(missing_docs)]
//! Sequentially-chained prompts — ports `@clack/prompts` `group()`.
//!
//! Each step is a closure that receives the map of results so far. The whole
//! group bails out on the first cancellation unless `on_cancel` returns true.
//!
//! # Example
//! ```rust,no_run
//! use cli_ui::prompt::{group, text, confirm};
//!
//! let answers = group()
//!     .step("name",   |_| Ok(text("Your name").run()?))
//!     .step("public", |_| Ok(confirm("Make profile public?").run()?.to_string()))
//!     .run()
//!     .unwrap();
//!
//! println!("{:?}", answers.get("name"));
//! ```

use super::error::{PromptError, Result};
use std::collections::BTreeMap;

type Answers = BTreeMap<String, String>;
type StepFn = Box<dyn FnOnce(&Answers) -> Result<String>>;

pub struct GroupBuilder {
    steps: Vec<(String, StepFn)>,
}

pub fn group() -> GroupBuilder {
    GroupBuilder { steps: Vec::new() }
}

impl GroupBuilder {
    /// Add a step. The closure may inspect previously collected answers.
    pub fn step<F>(mut self, name: impl Into<String>, f: F) -> Self
    where
        F: FnOnce(&Answers) -> Result<String> + 'static,
    {
        self.steps.push((name.into(), Box::new(f)));
        self
    }

    pub fn run(self) -> Result<Answers> {
        let mut answers = Answers::new();
        for (name, step) in self.steps {
            match step(&answers) {
                Ok(v) => {
                    answers.insert(name, v);
                }
                Err(PromptError::Interrupted) => {
                    super::cancel("Cancelled");
                    return Err(PromptError::Interrupted);
                }
                Err(e) => return Err(e),
            }
        }
        Ok(answers)
    }
}
