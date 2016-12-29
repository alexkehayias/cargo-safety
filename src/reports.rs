use std::collections::{HashSet};
use checks::{UnsafeCode};

#[derive(Debug, Serialize)]
pub struct SafetyReport {
    repo_url: String,
    status: bool,
    offenses: HashSet<UnsafeCode>,
}

impl SafetyReport {
    pub fn new(repo_url: String,
               status: bool,
               offenses: HashSet<UnsafeCode>) -> SafetyReport {
        SafetyReport {repo_url: repo_url, status: status, offenses: offenses}
    }
}
