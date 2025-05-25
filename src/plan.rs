use serde::Deserialize;
use std::{collections::HashMap, fmt, fs};

#[derive(Deserialize, Debug)]
pub struct TestPlan {
    #[serde(rename = "in")]
    pub input: Input,
    #[serde(default)]
    pub plan: Plan,
    #[serde(rename = "out")]
    pub output: Output,
}

#[derive(Deserialize, Debug, Default)]
pub struct Plan {
    #[serde(default = "default_exec")]
    pub exec: String,
}

fn default_exec() -> String {
    "curl".to_string()
}

#[derive(Deserialize, Debug)]
pub struct Input {
    #[serde(default = "default_method")]
    pub method: Option<String>,
    pub json: Option<String>,
    pub url: String,
}

fn default_method() -> Option<String> {
    Some("GET".to_string())
}

#[derive(Deserialize, Debug)]
pub struct Output {
    pub expect: HashMap<String, String>,
}

impl TestPlan {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

impl fmt::Display for TestPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "testplan:\ninput: {:?}\nplan: {:?}\noutput: {:?}",
            self.input, self.plan, self.output
        )
    }
}
