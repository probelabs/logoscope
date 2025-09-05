use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster {
    pub template: String,
    pub count: usize,
}

pub fn cluster_masked(lines: &[String]) -> Vec<Cluster> {
    let mut map: BTreeMap<String, usize> = BTreeMap::new();
    for line in lines {
        let template = to_template(line);
        *map.entry(template).or_insert(0) += 1;
    }
    map.into_iter()
        .map(|(template, count)| Cluster { template, count })
        .collect()
}

fn to_template(masked: &str) -> String {
    // Replace typed placeholders with generic <*> for clustering
    masked
        .replace("<NUM>", "<*>")
        .replace("<IP>", "<*>")
        .replace("<EMAIL>", "<*>")
        .replace("<TIMESTAMP>", "<*>")
}

