# GitUI 的 NSH DiffView 补丁

此目录来自 crates.io 的 Apache-2.0 `nsh-base 0.6.0`，对应 [NSH 源码版本](https://github.com/longbridge/nsh/tree/554163758e30d2f62f814096a6327937dcb4fe49/crates/nsh-base)。根目录的 Cargo 配置将所有 `nsh-base 0.6.0` 引用解析到这里，`nsh run` 生成的 Cargo 工程也使用同一份补丁。更新 NSH 时应核对上游是否已覆盖以下行为：

- Split 只有最近接收真实横向输入的一侧可以向对侧发送滚动命令。触控板和滚动条通过拖动开始事件确定主动侧，横向滚轮通过滚轮输入确定主动侧。跟随侧的偏移报告不会反向回写。
- Blame 源码视图可订阅 DiffView 纵向滚动输入事件，用于确定归属列和源码列同步时的主动侧。
- 行号列只读取行号和行状态，避免重复解析正文的着色和行内差异。可使用 `RUST_LOG=nsh_base::diff::scroll=trace nsh run` 记录 DiffView 的行生成范围和横向回写。
- 实际渲染行数不超过 1,000 时一次创建全部视图行；超过预算后按视口创建。Split 两侧与软换行行数都会计入预算，语法高亮仍只处理视口附近的内容。
