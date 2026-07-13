//! 禅道v9.0.3 命令行客户端库。

// 骨架阶段：大量端口、DTO 和适配器已定义但尚未被业务命令使用。
// 随着子任务 03～06 实现，这些警告会自然消失。
#![allow(dead_code)]

pub mod adapters;
pub mod application;
pub mod cli;
pub mod domain;
pub mod infrastructure;
