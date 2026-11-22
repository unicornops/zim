//! Zim's desktop application entry point.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    zim_desktop::run();
}
