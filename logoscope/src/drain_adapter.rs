use thiserror::Error;

#[derive(Debug, Error)]
pub enum DrainError {
    #[error("drain operation error: {0}")]
    Generic(String),
}

pub struct DrainAdapter {
    tree: drain_rs::DrainTree,
}

#[derive(Debug, Clone)]
pub struct DrainCluster {
    pub template: String,
    pub size: usize,
}

impl DrainAdapter {
    pub fn new_default() -> Self {
        Self { tree: Default::default() }
    }

    pub fn from_tree(tree: drain_rs::DrainTree) -> Self {
        Self { tree }
    }

    pub fn as_tree(&self) -> &drain_rs::DrainTree {
        &self.tree
    }

    pub fn into_tree(self) -> drain_rs::DrainTree {
        self.tree
    }

    pub fn insert(&mut self, line: &str) -> Result<(), DrainError> {
        let _ = self
            .tree
            .add_log_line(line)
            .ok_or_else(|| DrainError::Generic("failed to add log line".into()))?;
        Ok(())
    }

    pub fn insert_and_get_template(&mut self, line: &str) -> Result<String, DrainError> {
        let lg = self
            .tree
            .add_log_line(line)
            .ok_or_else(|| DrainError::Generic("failed to add log line".into()))?;
        Ok(to_generic_template(&lg.as_string()))
    }

    pub fn insert_and_get_template_raw(&mut self, line: &str) -> Result<String, DrainError> {
        let lg = self
            .tree
            .add_log_line(line)
            .ok_or_else(|| DrainError::Generic("failed to add log line".into()))?;
        Ok(lg.as_string())
    }

    pub fn clusters(&self) -> Vec<DrainCluster> {
        self.tree
            .log_groups()
            .into_iter()
            .map(|lg| DrainCluster {
                template: to_generic_template(&lg.as_string()),
                size: lg.num_matched() as usize,
            })
            .collect()
    }
}

pub fn to_generic_template(s: &str) -> String {
    s.replace("<NUMBER>", "<*>")
        .replace("<IPV4>", "<*>")
        .replace("<IPV6>", "<*>")
        .replace("<NUM>", "<*>")
        .replace("<IP>", "<*>")
        .replace("<TIMESTAMP>", "<*>")
        .replace("<EMAIL>", "<*>")
}
