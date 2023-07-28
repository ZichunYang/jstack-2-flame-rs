use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::process::{Command, Output};
use std::sync::RwLock;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use regex::Regex;

lazy_static! {
    static ref GLOBAL_NODE: RwLock<FlameGraphNode> = RwLock::new(FlameGraphNode::new("Root", 0));
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Process {
    pub pid: u32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlameGraphNode {
    pub name: String,
    pub num: u32,
    pub children: HashMap<String, FlameGraphNode>,
}

impl FlameGraphNode {
    fn new(name: &str, num: u32) -> Self {
        FlameGraphNode {
            name: name.to_string(),
            num,
            children: HashMap::new(),
        }
    }

    fn get_or_insert(&mut self, name: &str) -> &mut Self {
        let child = self.children.entry(name.to_string()).or_insert_with(|| Self::new(name, 1));
        child
    }

    pub fn merge(&mut self, other: FlameGraphNode) {
        self.num += other.num;

        for (name, other_child) in other.children {
            match self.children.entry(name) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().merge(other_child);
                }
                Entry::Vacant(entry) => {
                    entry.insert(other_child);
                }
            }
        }
    }
}

pub fn get_java_processes() -> Result<Vec<Process>, String> {
    let mut processes: Vec<Process> = Vec::new();
    let output = Command::new("jps")
        .arg("-l")
        .output()
        .map_err(|e| format!("Failed to execute jps command: {}", e))?;

    if !output.status.success() {
        return Err(format!("jps command failed with exit code: {}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 {
            continue;
        }
        let pid: u32 = parts[0].parse().map_err(|e| format!("Failed to parse PID: {}", e))?;
        let name = parts[1].to_string();
        processes.push(Process { pid, name });
    }

    Ok(processes)
}

fn parse_stack_trace(lines: Vec<&str>) -> FlameGraphNode {
    let mut root = FlameGraphNode::new("Root", 1);
    let name = lines[0].split("\"").collect::<Vec<_>>()[1];
    let re = Regex::new(r"[0-9]+").unwrap();
    let name = re.replace_all(&name, "{num}");
    let child = root.get_or_insert(&name);

    let mut current_node = child;
    for &line in lines[1..].iter().rev() {
        if line.is_empty() {
            continue;
        }
        let trimmed = line.trim_start().replace("\t", "");
        if !trimmed.starts_with("at")
            && !trimmed.starts_with("-") {
            continue;
        }
        let trimmed = trimmed.strip_prefix("-").unwrap_or(&trimmed).trim();
        let trimmed = trimmed.strip_prefix("at").unwrap_or(&trimmed).trim();
        current_node = current_node.get_or_insert(trimmed);
    }
    root
}

pub fn clear_jstack_info() {
    *GLOBAL_NODE.write().unwrap() = FlameGraphNode::new("Root", 0);
}

// Parse the jstack info
pub fn parse_jstack_info(pid: &str) -> Result<FlameGraphNode, Box<dyn std::error::Error>> {
    let mut root = GLOBAL_NODE.write().unwrap();

    // Execute the jstack command
    let output: Output = Command::new("jstack")
        .arg(pid.to_string())
        .output()?;

    // Check if the command ran successfully
    // if !output.status.success() {
    //     let err = String::from_utf8_lossy(&output.stderr);
    // return Err(format!("jstack command failed: {}", err).into());
    // }

    // Convert the output to a string
    let jstack_info = String::from_utf8_lossy(&output.stdout);

    // This example assumes that separate stack traces are separated by two newlines.
    if let Some(index) = jstack_info.find('"') {
        // Add 1 to the index to skip the '"' itself
        let substring = &jstack_info[index..];
        let strings = substring
            .split("\"VM Thread").collect::<Vec<_>>()[0]
            .split("\n").collect::<Vec<_>>();
        let split_strings = split_strings(strings);
        for line in split_strings {
            if line.is_empty() {
                continue;
            }
            let parsed = parse_stack_trace(line);
            root.merge(parsed)
        }
    }
    Ok(root.clone())
}

fn split_strings(input: Vec<&str>) -> Vec<Vec<&str>> {
    let mut result: Vec<Vec<&str>> = Vec::new();
    let mut current_line: Vec<&str> = Vec::new();

    for line in input {
        if !line.starts_with('\t')
            && !current_line.is_empty()
            && !line.starts_with(' ') {
            if !current_line.is_empty() {
                result.push(current_line);
            }
            current_line = Vec::new();
        }
        if !line.is_empty() {
            current_line.push(line);
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    result
}