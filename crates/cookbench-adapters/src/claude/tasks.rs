use serde_json::Value;

/// Bounded structured task progress extracted from Claude's TodoWrite/task
/// payloads. Content is intentionally discarded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskProgress {
    pub completed: u32,
    pub total: u32,
}

pub fn extract_task_progress(record: &Value, max_items: usize) -> Option<TaskProgress> {
    let todos = find_todo_array(record, 0, 32)?;
    let bounded = todos.iter().take(max_items);
    let mut total = 0_u32;
    let mut completed = 0_u32;
    for todo in bounded {
        let status = todo.get("status")?.as_str()?;
        total = total.checked_add(1)?;
        if matches!(status, "completed" | "complete" | "done") {
            completed = completed.checked_add(1)?;
        }
    }
    (total > 0).then_some(TaskProgress { completed, total })
}

fn find_todo_array(value: &Value, depth: usize, max_depth: usize) -> Option<&Vec<Value>> {
    if depth > max_depth {
        return None;
    }
    let object = value.as_object()?;
    for key in ["todos", "tasks"] {
        if let Some(items) = object.get(key).and_then(Value::as_array) {
            return Some(items);
        }
    }
    for child in object.values() {
        if let Some(todos) = find_todo_array(child, depth + 1, max_depth) {
            return Some(todos);
        }
    }
    None
}
