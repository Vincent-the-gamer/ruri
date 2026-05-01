use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Definition of a tool that can be made available to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    pub function: ToolFunction,
}

/// Type of tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Function,
}

/// Definition of a function tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<ToolParameters>,
}

/// JSON Schema style parameter definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    pub schema_type: SchemaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, ToolParameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Schema type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    Object,
}

/// A single parameter definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    #[serde(rename = "type")]
    pub param_type: ParameterType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// Parameter type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    String,
    Number,
    Integer,
    Boolean,
    Array,
    Object,
}

// ─── Builder helpers ────────────────────────────────────────────────

impl ToolDefinition {
    pub fn function(name: impl Into<String>) -> ToolDefinitionBuilder {
        ToolDefinitionBuilder {
            name: name.into(),
            description: None,
            parameters: None,
        }
    }
}

pub struct ToolDefinitionBuilder {
    name: String,
    description: Option<String>,
    parameters: Option<ToolParameters>,
}

impl ToolDefinitionBuilder {
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn parameter(
        self,
        name: impl Into<String>,
        param_type: ParameterType,
        required: bool,
    ) -> Self {
        self.parameter_with_description(name, param_type, required, None::<String>)
    }

    pub fn parameter_with_description(
        mut self,
        name: impl Into<String>,
        param_type: ParameterType,
        required: bool,
        description: Option<impl Into<String>>,
    ) -> Self {
        let param_name: String = name.into();
        let params = self.parameters.get_or_insert_with(|| ToolParameters {
            schema_type: SchemaType::Object,
            properties: Some(HashMap::new()),
            required: Some(Vec::new()),
        });

        if let Some(ref mut props) = params.properties {
            props.insert(
                param_name.clone(),
                ToolParameter {
                    param_type,
                    description: description.map(|d| d.into()),
                    enum_values: None,
                },
            );
        }

        if required && let Some(ref mut req) = params.required {
            req.push(param_name);
        }

        self
    }

    pub fn build(self) -> ToolDefinition {
        ToolDefinition {
            tool_type: ToolType::Function,
            function: ToolFunction {
                name: self.name,
                description: self.description,
                parameters: self.parameters,
            },
        }
    }
}
