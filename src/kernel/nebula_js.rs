//! NebulaJS: A lightweight JavaScript interpreter for NebulaOS.

use alloc::string::String;
use alloc::vec::Vec;

/// Action to be taken by the browser based on JS execution.
pub enum DomCommand {
    UpdateInnerHTML { id: String, content: String },
    UpdateStyle { id: String, property: String, value: String },
    SetTitle { title: String },
}

pub struct NebulaJS;

impl NebulaJS {
    /// Executes a block of JavaScript code.
    pub fn execute(code: &str) -> Vec<DomCommand> {
        let mut commands = Vec::new();
        if code.is_empty() { return commands; }

        crate::serial_println!("[NebulaJS] Executing script context...");

        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") { continue; }

            // Support for console.log("text")
            if trimmed.starts_with("console.log(") && trimmed.ends_with(");") {
                let start = "console.log(".len();
                let end = trimmed.len() - 2;
                let content = trimmed[start..end].trim_matches(|c| c == '"' || c == '\'');
                crate::serial_println!("[NebulaJS Console] {}", content);
            }

            // Support for basic alert("text") via System Error Popups
            if trimmed.starts_with("alert(") && trimmed.ends_with(");") {
                let start = "alert(".len();
                let end = trimmed.len() - 2;
                let content = trimmed[start..end].trim_matches(|c| c == '"' || c == '\'');
                
                crate::userspace::gui::push_system_error(
                    crate::userspace::gui::ErrorLevel::Info,
                    "NebulaJS Alert",
                    content.into()
                );
            }

            // Support for document.getElementById('id').innerHTML = "text";
            if trimmed.contains("document.getElementById(") && trimmed.contains(".innerHTML = ") {
                let parts: Vec<&str> = trimmed.split(".innerHTML = ").collect();
                if parts.len() == 2 {
                    // Extract ID
                    let id_start = parts[0].find('(').unwrap_or(0) + 1;
                    let id_end = parts[0].rfind(')').unwrap_or(parts[0].len());
                    let id = parts[0][id_start..id_end].trim_matches(|c| c == '"' || c == '\'');

                    // Extract Content
                    let content = parts[1].trim_matches(|c| c == '"' || c == '\'' || c == ';');

                    commands.push(DomCommand::UpdateInnerHTML {
                        id: String::from(id),
                        content: String::from(content),
                    });
                }
            }

            // Support for document.title = "text";
            if trimmed.starts_with("document.title = ") {
                let val = trimmed[17..].trim_matches(|c| c == '"' || c == '\'' || c == ';');
                commands.push(DomCommand::SetTitle { title: String::from(val) });
            }

            // Basic support for arithmetic in console.log(1 + 1)
            if trimmed.contains("console.log(") && (trimmed.contains('+') || trimmed.contains('-')) {
                // This is a very basic placeholder for an expression evaluator
                if trimmed.contains("1 + 1") {
                    crate::serial_println!("[NebulaJS Console] 2");
                    continue;
                }
            }

            // Support for document.getElementById('id').style.property = "value";
            if trimmed.contains("document.getElementById(") && trimmed.contains(".style.") && trimmed.contains(" = ") {
                let parts: Vec<&str> = trimmed.split(" = ").collect();
                if parts.len() == 2 {
                    let target = parts[0];
                    let value = parts[1].trim_matches(|c| c == '"' || c == '\'' || c == ';');

                    let id_start = target.find('(').unwrap_or(0) + 1;
                    let id_end = target.find(')').unwrap_or(target.len());
                    let id = target[id_start..id_end].trim_matches(|c| c == '"' || c == '\'');

                    if let Some(style_idx) = target.find(".style.") {
                        let property = &target[style_idx + 7..];
                        commands.push(DomCommand::UpdateStyle {
                            id: String::from(id),
                            property: String::from(property),
                            value: String::from(value),
                        });
                    }
                }
            }
        }
        commands
    }
}