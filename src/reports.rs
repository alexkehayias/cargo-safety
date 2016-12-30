use std::collections::{HashSet};
use checks::{UnsafeCode};


#[allow(non_camel_case_types)]
#[derive(Debug, Serialize)]
pub enum Status {
    passed,
    failed,
}

impl Status {
    pub fn from_bool(pass: bool) -> Status {
        if pass {
            Status::passed
        } else {
            Status::failed
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SafetyReport {
    repo_url: String,
    status: Status,
    offenses: HashSet<UnsafeCode>,
}

impl SafetyReport {
    pub fn new(repo_url: String,
               status: Status,
               offenses: HashSet<UnsafeCode>) -> SafetyReport {
        SafetyReport {repo_url: repo_url, status: status, offenses: offenses}
    }
}
