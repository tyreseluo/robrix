spec: task
name: "Room Input Toolbar Redesign + Emoji Expansion"
inherits: project
tags: [feature, ui, room-input, emoji]
estimate: 2d
---

## Intent

将房间输入区改成与参考图一致的结构与交互方向：输入框在最上方，功能按钮独立为下一行；按钮触发的面板统一向上展开。并扩充可快捷插入的 emoji 数量，重排“更多”弹窗为两行网格入口。

## Constraints

- 保持现有消息发送、附件上传、定位预览、房间信息、线程入口等已有业务行为不变
- 仅调整 `RoomInputBar` 相关 UI 结构与必要事件处理，不改 Matrix 请求协议与数据结构
- 新增功能按钮若暂无后端能力，可使用统一提示（例如 “Coming soon”）
- 不引入新依赖，不新增复杂状态机

## Decisions

- 输入区改为两层：
  - 上层仅展示可多行输入的 `MentionableTextInput`
  - 下层为横向功能按钮栏（emoji / 语音 / 文件 / 翻译 Aa / 更多）
- `emoji` 与 `更多` 面板默认收起，点击对应按钮后在按钮栏上方展开
- 同一时刻最多展示一个主弹层（emoji、更多）
- emoji 快捷面板扩展到至少 14 个常用表情
- “更多”弹窗改为 2x4 网格视觉布局，保留已有可用功能入口：
  - 文件上传
  - 房间信息
  - 位置
  - 线程
  - 其余入口可先提供占位能力（点击提示）

## Boundaries

### Allowed Changes
- `src/room/room_input_bar.rs`
- `specs/task-room-input-toolbar-redesign.spec.md`

### Forbidden
- 不修改 `sliding_sync.rs` 等消息发送管线
- 不改动与输入区无关的页面布局
- 不运行 `cargo fmt` / `rustfmt`

## Acceptance Criteria

Scenario: 输入框位于最上方，功能按钮独立成下一行
  Test: manual_test_room_input_toolbar_vertical_layout
  Given 用户进入任意可发送消息的房间
  When 房间输入区渲染完成
  Then 输入框显示在功能按钮行上方
  And 功能按钮行为独立水平一行

Scenario: emoji 面板向上展开且可插入扩展表情
  Test: manual_test_room_input_emoji_panel_upward_and_extended
  Given 用户位于房间输入区
  When 点击 emoji 按钮
  Then emoji 面板在按钮行上方展开
  And 面板内可见至少 14 个常用 emoji
  When 用户点击任一 emoji
  Then 对应 emoji 被追加到输入框文本
  And emoji 面板自动关闭

Scenario: 更多面板向上展开并使用两行网格
  Test: manual_test_room_input_more_panel_grid_layout
  Given 用户位于房间输入区
  When 点击“更多（+）”按钮
  Then 更多面板在按钮行上方展开
  And 面板以两行网格方式展示功能入口
  And 每个入口包含图标与文字标签

Scenario: 可用入口行为保持不变
  Test: manual_test_room_input_more_panel_existing_actions
  Given 更多面板已展开
  When 点击“文件”入口
  Then 触发文件选择流程
  When 点击“位置”入口
  Then 显示位置预览流程
  When 点击“房间信息”或“线程”入口
  Then 触发原有对应面板/页面动作

Scenario: 主弹层互斥
  Test: manual_test_room_input_popups_mutual_exclusive
  Given emoji 面板已展开
  When 点击“更多（+）”按钮
  Then emoji 面板关闭
  And 更多面板展开

## Out of Scope

- 语音消息真实录制与发送能力
- 截图中“红包/日程/任务/边写边译”等完整业务功能实现
- 输入区之外的全局主题或字号体系重构
