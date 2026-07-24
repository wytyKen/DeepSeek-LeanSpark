# 图标说明

Tauri 打包需要图标文件。请用以下命令从一张源图生成全套图标：

```bash
# 安装 tauri-cli（如尚未安装）
cargo install tauri-cli --version "^2"

# 在 DeepSeek-LeanSpark/ 目录下执行
cargo tauri icon path/to/your-icon.png
```

生成的图标会自动放到 `src-tauri/icons/` 目录。

如未生成图标，`cargo tauri dev` 仍可运行（开发模式不强制图标），但 `cargo tauri build` 需要图标。

建议源图为 1024x1024 PNG，内容为 DeepSeek-LeanSpark 品牌 logo。
