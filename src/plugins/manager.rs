//! 外掛指令擴充系統 (PluginManager)
//!
//! 允許透過註冊指令擴充 AlexCode 的工具選單與命令面板。

use std::collections::HashMap;

pub type CommandAction = Box<dyn Fn() -> String + Send + Sync>;

#[derive(Default)]
pub struct PluginManager {
    commands: HashMap<String, CommandAction>,
    command_order: Vec<String>,
}

impl PluginManager {
    pub fn new() -> Self {
        let mut mgr = Self::default();
        mgr.register_builtin_plugins();
        mgr
    }

    pub fn register_command<F>(&mut self, name: &str, action: F)
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        if !self.commands.contains_key(name) {
            self.command_order.push(name.to_string());
        }
        self.commands.insert(name.to_string(), Box::new(action));
    }

    pub fn run_command(&self, name: &str) -> Option<String> {
        self.commands.get(name).map(|action| action())
    }

    pub fn command_names(&self) -> &[String] {
        &self.command_order
    }

    fn register_builtin_plugins(&mut self) {
        self.register_command("插入目前日期時間", || {
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
        });

        self.register_command("產生 UUID", || {
            // 輕量偽隨機 UUID 格式
            let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
            format!("{:08x}-4000-8000-{:012x}", (now & 0xFFFFFFFF), (now >> 32))
        });
    }
}
