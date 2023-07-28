// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use java::jstack::Process;

use crate::java::jstack;
use crate::java::jstack::{FlameGraphNode, parse_jstack_info};

pub mod java {
    pub mod jstack;
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet() -> Vec<Process> {
    let processes = jstack::get_java_processes().unwrap();
    processes
}

#[tauri::command]
fn get_jstack_info(pid: &str) -> FlameGraphNode {
    parse_jstack_info(pid).unwrap()
}

#[tauri::command]
fn clear_jstack_info() {
    jstack::clear_jstack_info();
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            get_jstack_info,
            clear_jstack_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
