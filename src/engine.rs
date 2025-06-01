use log::{debug, error, info};
use regex::Regex;
use serde_json::Value;
use std::{
    collections::HashMap,
    env,
    error::Error,
    process::{Command, Output},
};

use crate::plan::TestPlan;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("API error {0}: {1}")]
    ApiError(i64, String),
    #[error("validation failed - missing field: {0}")]
    MissingField(String),
    #[error("validation failed - field mismatch: {0}")]
    FieldMismatch(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("failed to parse integer: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

#[derive(Debug)]
pub struct TestEngine {
    plan: TestPlan,
    env_vars: HashMap<String, String>,
}

impl TestEngine {
    pub fn new(plan: TestPlan) -> Self {
        TestEngine {
            plan,
            env_vars: HashMap::new(),
        }
    }

    pub fn execute(&mut self) -> Result<(), Box<dyn Error>> {
        let executor = self.plan.plan.executor.as_str();
        debug!("using {executor} executor");

        let output = match executor {
            "curl" | "" => self.execute_curl()?,
            _ => {
                return Err(Box::new(EngineError::ExecutionFailed(format!(
                    "unsupported {executor} executor"
                ))));
            }
        };

        let response = String::from_utf8(output.stdout.clone())?;
        debug!("raw response: {}", response);
        let json: Value = serde_json::from_str(&response)?;

        self.validate_command_output(&output)?;
        self.validate_output(&json)?;
        self.assign_output(&json)?;
        Ok(())
    }

    fn execute_curl(&self) -> Result<Output, Box<dyn Error>> {
        let mut cmd = Command::new(self.plan.plan.executor.clone());
        let mut cmd = cmd
            .arg("-sS")
            .arg("-X")
            .arg(self.plan.input.method.as_deref().unwrap_or_default())
            .arg("-d")
            .arg(self.substitute_env_vars(self.plan.input.json.as_deref().unwrap_or(""))?)
            .arg("-H")
            .arg("Content-Type:application/json");

        cmd = if let Ok(token) = env::var("TSTIT_TKN") {
            cmd.arg("-H").arg(format!("Authorization:{}", token))
        } else {
            cmd
        };

        let url = format!(
            "{}{}",
            env::var("TSTIT_URL").map_err(|_| "TSTIT_URL env var is not set!")?,
            self.substitute_env_vars(&self.plan.input.url)?
        );
        let cmd = cmd.arg(url);

        debug!("executing command: {:?}", cmd);
        let output = cmd.output()?;
        debug!("output: {output:?}");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("command failed: {}", stderr);
            return Err(Box::new(EngineError::ExecutionFailed(format!(
                "command failed with status: {}",
                output.status
            ))));
        }

        debug!("command completed with: {}", output.status);
        Ok(output)
    }

    fn validate_command_output(&self, output: &Output) -> Result<(), Box<dyn Error>> {
        if output.stdout.is_empty() {
            return Err(Box::new(EngineError::InvalidResponse(
                "empty response, is service down?".to_string(),
            )));
        }
        Ok(())
    }

    fn validate_output(&self, json: &Value) -> Result<(), Box<dyn Error>> {
        // validating the mandatory code field
        match json.get("code").and_then(Value::as_i64) {
            Some(0) => {}
            Some(code) => {
                let error_msg = json
                    .get("data")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "unknown error".to_string());
                return Err(Box::new(EngineError::ApiError(code, error_msg)));
            }
            None => {
                return Err(Box::new(EngineError::MissingField(
                    "required field 'code' is missing".to_string(),
                )));
            }
        }

        // special handling for direct data field validation (PATCH case)
        if let Some(expected_data) = self.plan.output.expect.get("data") {
            match json.get("data") {
                Some(actual_data) => {
                    let expected_data_substituted = self.substitute_env_vars(expected_data)?;
                    if !self.compare_values(actual_data, &expected_data_substituted)? {
                        return Err(Box::new(EngineError::FieldMismatch(format!(
                            "'data' field expected '{}', but got '{}'",
                            expected_data_substituted, actual_data
                        ))));
                    }
                    if self.plan.output.expect.len() == 1 {
                        info!("validation successful");
                        return Ok(());
                    }
                }
                None => {
                    return Err(Box::new(EngineError::MissingField(
                        "required field 'data' is missing".to_string(),
                    )));
                }
            }
        }

        // validating other fields (GET case)
        let validation_target = match json.get("data") {
            Some(Value::Object(obj)) => obj,
            Some(_) => json.as_object().unwrap(),
            None => json.as_object().unwrap(),
        };

        for (key, expected_value) in &self.plan.output.expect {
            if key == "code" || key == "data" {
                continue;
            }

            match validation_target.get(key) {
                Some(value) => {
                    let expected_value_substituted = self.substitute_env_vars(expected_value)?;
                    if !self.compare_values(value, &expected_value_substituted)? {
                        return Err(Box::new(EngineError::FieldMismatch(format!(
                            "field '{}' expected '{}' but got '{}'",
                            key, expected_value_substituted, value
                        ))));
                    }
                }
                None => {
                    return Err(Box::new(EngineError::MissingField(format!(
                        "required field '{}' is missing",
                        key
                    ))));
                }
            }
        }

        info!("validation successful");
        Ok(())
    }

    fn compare_values(&self, value: &Value, expected: &str) -> Result<bool, Box<dyn Error>> {
        debug!("compare_values: {value} and \"{expected}\"");
        match value {
            Value::Number(n) => {
                let actual = n.as_i64().ok_or("integer expected")?;
                if expected.starts_with('>') {
                    println!(
                        "actual: {actual}, expected: {}",
                        expected[1..].parse::<i64>()?
                    );
                    Ok(actual > expected[1..].parse::<i64>()?)
                } else if expected.starts_with('<') {
                    Ok(actual < expected[1..].parse::<i64>()?)
                } else {
                    Ok(actual == expected.parse::<i64>()?)
                }
            }
            Value::String(s) => Ok(s == expected),
            Value::Bool(b) => match expected {
                "true" => Ok(*b),
                "false" => Ok(!*b),
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    fn assign_output(&mut self, json: &Value) -> Result<(), Box<dyn Error>> {
        if let Some(assign_map) = &self.plan.output.assign {
            for (key, var_name) in assign_map {
                if let Some(value) = json.get(key) {
                    let string_value = value.to_string().replace("\"", "");
                    unsafe {
                        env::set_var(var_name.trim_start_matches('$'), &string_value);
                    }
                    self.env_vars.insert(var_name.clone(), string_value.clone());
                    info!("assigned {string_value} to {var_name} var");
                }
            }
        }
        Ok(())
    }

    fn substitute_env_vars(&self, text: &str) -> Result<String, Box<dyn Error>> {
        let re = Regex::new(r"\$[A-Za-z0-9_]+").unwrap();
        let mut result = text.to_string();

        for cap in re.captures_iter(text) {
            let var_name = cap.get(0).unwrap().as_str();
            let env_var_name = var_name.trim_start_matches('$');

            let env_value = env::var(env_var_name).or_else(|_| {
                self.env_vars
                    .get(var_name)
                    .map(|s| s.clone())
                    .ok_or(std::env::VarError::NotPresent)
            });

            match env_value {
                Ok(value) => {
                    result = result.replace(var_name, &value);
                }
                Err(_) => {
                    return Err(Box::new(EngineError::MissingField(format!(
                        "env var {} not found",
                        var_name
                    ))));
                }
            }
        }

        Ok(result)
    }
}
