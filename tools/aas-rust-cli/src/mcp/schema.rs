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
        "description": "[MANDATORY FIRST STEP] You MUST always use this tool to search for skills before proceeding with any task. Never rely on your own knowledge of what skills exist — always call search_skills with keywords derived from the user's request, then examine every returned result (name, description, risk level) before deciding which skills to load. Do not skip results just because the first few seem relevant. Only after searching should you use get_skill to fetch full content for matching skills.",
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
        "description": "[LOAD A SKILL] Fetch the full SKILL.md content for a skill identified via search_skills. Use this to load skills whose scope matches the task — even partially. Once loaded, follow the skill's workflow; its instructions supersede your own approach for that aspect of the work. Set include_content=false to get metadata only (faster), or true (default) to get full content for applying the skill's process.",
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
        "description": "List all skill categories with the number of skills in each category. Use this when search_skills returns no relevant results — browse categories to discover related skills you may have missed. Category names from the output can be used as filters in filter_skills or as refined search keywords.",
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
        "description": "[STRUCTURED FILTER] Filter skills by exact category, risk level, or tags. Use this when the user specifies concrete criteria (e.g., 'list all critical-risk skills') rather than free-text keywords. Can be used after search_skills returns no results — try filtering by a known category or risk level to discover relevant skills. Always prefer search_skills first for open-ended tasks.",
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
