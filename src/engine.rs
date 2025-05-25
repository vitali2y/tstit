use log::{debug, error, info};
use serde_json::Value;
use std::{
    env,
    error::Error,
    process::{Command, Output},
};

use crate::plan::TestPlan;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("command execution failed: {0}")]
    ExecutionFailed(String),
    // #[error("HTTP error {0}: {1}")]
    // HttpError(u16, String),
    #[error("API error {0}: {1}")]
    ApiError(i64, String),
    #[error("validation failed - missing field: {0}")]
    MissingField(String),
    #[error("validation failed - field mismatch: {0}")]
    FieldMismatch(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

pub struct TestEngine {
    plan: TestPlan,
}

impl TestEngine {
    pub fn new(plan: TestPlan) -> Self {
        TestEngine { plan }
    }

    pub fn execute(&self) -> Result<(), Box<dyn Error>> {
        let executor = self.plan.plan.exec.as_str();
        debug!("using executor: {}", executor);

        let output = match executor {
            "curl" | "" => self.execute_curl()?,
            _ => {
                return Err(Box::new(EngineError::ExecutionFailed(format!(
                    "unsupported executor: '{}'",
                    executor
                ))));
            }
        };

        self.validate_command_output(&output)?;
        self.validate_output(&output)
    }

    fn execute_curl(&self) -> Result<Output, Box<dyn Error>> {
        let mut cmd = Command::new("curl");
        let cmd = cmd
            .arg("-X")
            .arg(self.plan.input.method.as_deref().unwrap_or("GET"))
            .arg("-d")
            .arg(self.plan.input.json.as_deref().unwrap_or(""))
            .arg("-H")
            .arg("Content-Type:application/json");

        let cmd = if let Ok(token) = env::var("TKN") {
            cmd.arg("-H").arg(format!("Authorization:{}", token))
        } else {
            cmd
        };

        let url = format!(
            "{}{}",
            env::var("URL").map_err(|_| "URL env var is not set")?,
            self.plan.input.url
        );
        let cmd = cmd.arg(url);

        debug!("executing command: {:?}", cmd);
        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("command failed: {}", stderr);
            return Err(Box::new(EngineError::ExecutionFailed(format!(
                "curl failed with status: {}",
                output.status
            ))));
        }

        debug!("command completed with status: {}", output.status);
        Ok(output)
    }

    fn validate_command_output(&self, output: &Output) -> Result<(), Box<dyn Error>> {
        if output.stdout.is_empty() {
            return Err(Box::new(EngineError::InvalidResponse(
                "empty response - service may be down".into(),
            )));
        }
        Ok(())
    }

    fn validate_output(&self, output: &Output) -> Result<(), Box<dyn Error>> {
        let response = String::from_utf8(output.stdout.clone())?;
        debug!("raw response: {}", response);

        let json: Value = serde_json::from_str(&response)?;

        // validating the mandatory code field
        match json.get("code").and_then(Value::as_i64) {
            // success - proceed
            Some(0) => {}
            Some(code) => {
                let error_msg = json
                    .get("data")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "Unknown error".to_string());
                return Err(Box::new(EngineError::ApiError(code, error_msg)));
            }
            None => {
                return Err(Box::new(EngineError::MissingField(
                    "required field 'code' missing".to_string(),
                )));
            }
        }

        // special handling for direct data field validation (PATCH case)
        if let Some(expected_data) = self.plan.output.expect.get("data") {
            match json.get("data") {
                Some(actual_data) => {
                    if !self.compare_values(actual_data, expected_data)? {
                        return Err(Box::new(EngineError::FieldMismatch(format!(
                            "data field expected '{}', but got '{}'",
                            expected_data, actual_data
                        ))));
                    }
                    if self.plan.output.expect.len() == 1 {
                        info!("validation successful");
                        return Ok(());
                    }
                }
                None => {
                    return Err(Box::new(EngineError::MissingField(
                        "required field 'data' missing".to_string(),
                    )));
                }
            }
        }

        // validating other fields (GET case)
        let validation_target = match json.get("data") {
            Some(Value::Object(obj)) => obj,
            // using root if data isn't an object
            Some(_) => json.as_object().unwrap(),
            // usinge root if no data field
            None => json.as_object().unwrap(),
        };

        for (key, expected_value) in &self.plan.output.expect {
            if key == "code" || key == "data" {
                // skipping already validated fields
                continue;
            }

            match validation_target.get(key) {
                Some(value) => {
                    if !self.compare_values(value, expected_value)? {
                        return Err(Box::new(EngineError::FieldMismatch(format!(
                            "field '{}' expected '{}' but got '{}'",
                            key, expected_value, value
                        ))));
                    }
                }
                None => {
                    return Err(Box::new(EngineError::MissingField(format!(
                        "required field '{}' missing",
                        key
                    ))));
                }
            }
        }

        info!("validation successful");
        Ok(())
    }

    fn compare_values(&self, value: &Value, expected: &str) -> Result<bool, Box<dyn Error>> {
        match (value, expected) {
            (Value::Number(n), _) if expected.starts_with('>') => {
                let threshold = expected[1..].parse::<f64>()?;
                Ok(n.as_f64().unwrap_or(0.0) > threshold)
            }
            (Value::Number(n), _) if expected.starts_with('<') => {
                let threshold = expected[1..].parse::<f64>()?;
                Ok(n.as_f64().unwrap_or(0.0) < threshold)
            }
            (Value::Number(n), _) => Ok(n.as_f64().unwrap_or(0.0) == expected.parse::<f64>()?),
            (Value::String(s), _) => Ok(s == expected),
            (Value::Bool(b), "true") => Ok(*b),
            (Value::Bool(b), "false") => Ok(!*b),
            _ => Ok(false),
        }
    }
}
