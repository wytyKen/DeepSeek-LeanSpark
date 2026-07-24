use anyhow::Result;

pub fn load_system_prompt() -> Result<String> {
    // 编译期把 prompts/agent-prompt.md 内联进二进制
    let prompt = include_str!("../../prompts/agent-prompt.md");
    Ok(prompt.to_string())
}
