use serde_json::Value;

/// Return the list of tool definitions as JSON values matching the MCP spec.
pub fn tool_definitions() -> Vec<Value> {
    vec![
        search_skills_tool(),
        get_skill_tool(),
        list_categories_tool(),
        filter_skills_tool(),
    ]
}

fn search_skills_tool() -> Value {
    serde_json::json!({
        "name": "search_skills",
        "description": "Search the skills catalog by keyword. Returns matching skill metadata without loading full SKILL.md content.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Keywords to search across id, name, description, category, and tags fields." },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "default": 20 }
            },
            "required": ["query"]
        }
    })
}

fn get_skill_tool() -> Value {
    serde_json::json!({
        "name": "get_skill",
        "description": "Fetch the full SKILL.md content for a specific skill by id. Content is lazy-loaded from the local git store.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Exact skill identifier (e.g., 'brainstorming', '007')." },
                "include_content": { "type": "boolean", "default": true }
            },
            "required": ["id"]
        }
    })
}

fn list_categories_tool() -> Value {
    serde_json::json!({
        "name": "list_categories",
        "description": "List all skill categories with the number of skills in each category.",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

fn filter_skills_tool() -> Value {
    serde_json::json!({
        "name": "filter_skills",
        "description": "Filter skills by category, risk level, or tags without free-text search.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "category": { "type": "string", "description": "Filter by exact category name." },
                "risk": { "type": "string", "description": "Filter by risk level (safe, none, moderate, critical)." },
                "tags": { "type": "array", "items": { "type": "string" }, "description": "Skill must have ALL specified tags." },
                "limit": { "type": "integer", "minimum": 1, "maximum": 200, "default": 50 }
            },
            "required": []
        }
    })
}
