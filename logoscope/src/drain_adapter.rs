use thiserror::Error;
use crate::param_extractor;
use std::collections::HashMap;

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

    pub fn new_tuned(max_depth: u16, min_similarity: f32, max_children: u16) -> Self {
        let tree = drain_rs::DrainTree::new()
            .max_depth(max_depth)
            .min_similarity(min_similarity)
            .max_children(max_children);
        Self { tree }
    }

    pub fn new_tuned_with_filters(max_depth: u16, min_similarity: f32, max_children: u16) -> Self {
        let patterns = vec![
            "%{IPV4:IPV4}",
            "%{IPV6:IPV6}",
            "%{NUMBER:NUMBER}",
            "%{UUID:UUID}",
            "%{EMAILADDRESS:EMAIL}",
            "%{TIMESTAMP_ISO8601:TIMESTAMP}",
            "(?<HEX>[0-9a-fA-F]{16,})",
        ];
        let mut g = grok::Grok::with_patterns();
        let tree = drain_rs::DrainTree::new()
            .max_depth(max_depth)
            .min_similarity(min_similarity)
            .max_children(max_children)
            .filter_patterns(patterns)
            .build_patterns(&mut g);
        Self { tree }
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

    pub fn insert(&mut self, line: &str) -> Result<String, DrainError> {
        // Apply canonicalization before feeding to Drain
        let canonicalized = param_extractor::canonicalize_for_drain(line);
        let lg = self
            .tree
            .add_log_line(&canonicalized.masked_text)
            .ok_or_else(|| DrainError::Generic("failed to add log line".into()))?;
        Ok(lg.as_string())
    }

    /// Optimized insert that returns both template and canonicalization result to avoid redundant processing
    pub fn insert_with_canon(&mut self, line: &str) -> Result<(String, param_extractor::MaskingResult), DrainError> {
        // Apply canonicalization before feeding to Drain
        let canonicalized = param_extractor::canonicalize_for_drain(line);
        let lg = self
            .tree
            .add_log_line(&canonicalized.masked_text)
            .ok_or_else(|| DrainError::Generic("failed to add log line".into()))?;
        Ok((lg.as_string(), canonicalized))
    }

    pub fn insert_and_get_template(&mut self, line: &str) -> Result<String, DrainError> {
        // OPTIMIZATION: Use canonicalize_for_drain directly to avoid duplicate smart masking
        // canonicalize_for_drain already calls smart_mask_line internally, so we don't need to call it again
        let canonicalized = param_extractor::canonicalize_for_drain(line);
        
        // Check if canonicalization returned a high-confidence smart masking result
        // We can detect this by checking if the template has specific smart masking patterns
        let is_high_confidence_smart = canonicalized.masked_text.contains("<TIMESTAMP> <LOAD_BALANCER>") ||
                                       canonicalized.masked_text.contains("<CLIENT_IP> <REMOTE_LOGNAME>") ||
                                       canonicalized.masked_text.contains("<CLIENT_IP> - <REMOTE_USER>");
        
        if is_high_confidence_smart {
            // This is already a high-confidence smart template from canonicalization
            let _ = self.tree.add_log_line(&canonicalized.masked_text);
            return Ok(canonicalized.masked_text);
        }
        
        // Use the canonicalized result for drain processing
        let lg = self
            .tree
            .add_log_line(&canonicalized.masked_text)
            .ok_or_else(|| DrainError::Generic("failed to add log line".into()))?;
        Ok(to_generic_template(&lg.as_string()))
    }

    pub fn insert_and_get_template_raw(&mut self, line: &str) -> Result<String, DrainError> {
        // Apply canonicalization before feeding to Drain (smart masking is called inside canonicalize_for_drain)
        let canonicalized = param_extractor::canonicalize_for_drain(line);
        
        // Check if canonicalized data contains high-confidence smart masking result
        // canonicalize_for_drain already calls smart_mask_line internally, so we can check the result
        let is_smart_template = canonicalized.masked_text.contains("<TIMESTAMP> <LOAD_BALANCER>") ||
                                canonicalized.masked_text.contains("<CLIENT_IP> <REMOTE_LOGNAME>") ||
                                canonicalized.masked_text.contains("<CLIENT_IP> - <REMOTE_USER>");
        
        if is_smart_template {
            // This is already a high-confidence smart template from canonicalization
            let _ = self.tree.add_log_line(&canonicalized.masked_text);
            return Ok(canonicalized.masked_text);
        }
        
        let lg = self
            .tree
            .add_log_line(&canonicalized.masked_text)
            .ok_or_else(|| DrainError::Generic("failed to add log line".into()))?;
        Ok(lg.as_string())
    }

    /// Optimized version that accepts pre-computed canonicalization result to avoid redundant processing
    pub fn insert_and_get_template_raw_with_canon(
        &mut self, 
        canonicalized: &param_extractor::MaskingResult
    ) -> Result<String, DrainError> {
        // Check if canonicalized data contains high-confidence smart masking result
        // We can detect this by checking if the template has specific smart masking patterns
        let is_smart_template = canonicalized.masked_text.contains("<TIMESTAMP> <LOAD_BALANCER>") ||
                                canonicalized.masked_text.contains("<CLIENT_IP> <REMOTE_LOGNAME>") ||
                                canonicalized.masked_text.contains("<CLIENT_IP> - <REMOTE_USER>");
        
        if is_smart_template {
            // This is already a high-confidence smart template
            let _ = self.tree.add_log_line(&canonicalized.masked_text);
            return Ok(canonicalized.masked_text.clone());
        }
        
        // Use the pre-computed masked text directly
        let lg = self
            .tree
            .add_log_line(&canonicalized.masked_text)
            .ok_or_else(|| DrainError::Generic("failed to add log line".into()))?;
        Ok(lg.as_string())
    }

    /// Insert masked text directly into the Drain tree and return the template
    /// This method is optimized for batch processing where the same masked_text
    /// should only be inserted once to build the tree efficiently
    pub fn insert_masked(&mut self, masked_text: &str) -> Result<String, DrainError> {
        let lg = self
            .tree
            .add_log_line(masked_text)
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
    // Replace any <SOMETHING> pattern with <*>
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(s, "<*>").to_string()
}

/// Converts a raw drain template to a display template with descriptive placeholders
/// This keeps human-friendly placeholders like <API_ID> vs generic <*>
pub fn to_display_template(raw_template: &str, _original_line: &str) -> String {
    // For now, just return the raw template as it already contains descriptive placeholders
    // from the canonicalization process
    raw_template.to_string()
}

/// Computes clusters that can be merged by structural similarity
/// This provides post-Drain structural merging based on field patterns
pub fn clusters_merged_by_shape(clusters: &[DrainCluster]) -> Vec<DrainCluster> {
    let mut shape_groups: HashMap<String, Vec<&DrainCluster>> = HashMap::new();
    
    // Group clusters by their structural shape
    for cluster in clusters {
        let shape_key = compute_shape_key(&cluster.template);
        shape_groups.entry(shape_key).or_default().push(cluster);
    }
    
    // Create merged clusters from each shape group
    let mut merged_clusters = Vec::new();
    for (shape_key, group) in shape_groups {
        if group.len() == 1 {
            // Single cluster, no merging needed
            merged_clusters.push(group[0].clone());
        } else {
            // Multiple clusters with same shape - merge them
            let total_size: usize = group.iter().map(|c| c.size).sum();
            let merged_template = generalize_templates(group.iter().map(|c| &c.template).collect());
            
            merged_clusters.push(DrainCluster {
                template: merged_template.unwrap_or_else(|| shape_key.clone()),
                size: total_size,
            });
        }
    }
    
    merged_clusters.sort_by(|a, b| b.size.cmp(&a.size));
    merged_clusters
}

/// Computes a shape key for structural similarity comparison
/// This extracts the field structure pattern from a template
fn compute_shape_key(template: &str) -> String {
    // Extract the field names and their positions to create a structural signature
    let field_pattern_regex = regex::Regex::new(r"(\w+)\s*=\s*<[^>]+>").unwrap();
    let mut field_names: Vec<String> = field_pattern_regex
        .captures_iter(template)
        .map(|cap| cap.get(1).unwrap().as_str().to_string())
        .collect();
    
    field_names.sort();
    format!("fields:{}", field_names.join(","))
}

/// Generalizes multiple templates into a single representative template
fn generalize_templates(templates: Vec<&String>) -> Option<String> {
    if templates.is_empty() {
        return None;
    }
    if templates.len() == 1 {
        return Some(templates[0].clone());
    }
    
    // Find the common structure by tokenizing and comparing
    let tokenized: Vec<Vec<&str>> = templates.iter()
        .map(|t| t.split_whitespace().collect())
        .collect();
    
    if tokenized.is_empty() {
        return None;
    }
    
    let mut result_tokens = Vec::new();
    let first_template = &tokenized[0];
    
    // Compare token by token across all templates
    for (i, &token) in first_template.iter().enumerate() {
        let mut all_same = true;
        let mut all_placeholders = token.starts_with('<') && token.ends_with('>');
        
        for template_tokens in tokenized.iter().skip(1) {
            if i >= template_tokens.len() {
                all_same = false;
                break;
            }
            
            let other_token = template_tokens[i];
            if token != other_token {
                all_same = false;
            }
            
            if !(other_token.starts_with('<') && other_token.ends_with('>')) {
                all_placeholders = false;
            }
        }
        
        if all_same {
            result_tokens.push(token.to_string());
        } else if all_placeholders {
            // All tokens are placeholders but different - use generic placeholder
            result_tokens.push("<*>".to_string());
        } else {
            // Mixed or different non-placeholder tokens
            result_tokens.push("<*>".to_string());
        }
    }
    
    Some(result_tokens.join(" "))
}
