# getUserMedia 双应用消费测试页（M0-T3）

App B（Chromium 侧）的独立消费者探针：枚举 → 定位 `SWVCamMediaSource (Windows 虚拟摄像头)` →
`getUserMedia` 独占打开 → `requestVideoFrameCallback` 计帧测 FPS → 经文件通道受控 stop/start。

## 组成

| 文件 | 职责 |
|---|---|
| `index.html` | 测试页：自动定位虚拟摄像头并拉流；大字状态面板（STATE/DEVICE/SETTINGS/FRAMES/FPS/ERRORS）供截图留证；track `ended/mute/unmute` 事件带时间戳入事件栏；每 5s POST `/report`，1s 轮询 `/cmd` |
| `serve.py` | stdlib 控制服务：静态服务本页（`http://127.0.0.1` 为安全上下文）；`GET /cmd` 返回命令文件内容；`POST /report` 追加 jsonl 报告 |

## 本次 run 的启动方式（复现）

```bash
# 1) 控制服务（日志重定向到 run logs/serve.log）
python -I tests/fixtures/getusermedia/serve.py \
  --port 8093 \
  --cmdfile artifacts/G0/<run-id>/staging/chrome-cmd.txt \
  --reportfile artifacts/G0/<run-id>/logs/chrome-reports.jsonl \
  > artifacts/G0/<run-id>/logs/serve.log 2>&1 &

# 2) Chrome 独立会话（fresh profile，位于 run staging，测试后删除）
"C:/Program Files/Google/Chrome/Application/chrome.exe" \
  --user-data-dir=<run>/staging/chrome-profile \
  --no-first-run --no-default-browser-check \
  --use-fake-ui-for-media-stream \
  "http://127.0.0.1:8093/"
```

- `--use-fake-ui-for-media-stream`：仅自动点掉权限弹窗，**不伪造设备流**（媒体路径全真实）；与 profile 预注入权限双保险，首选路径下物理摄像头零触碰。
- 命令通道：向 `<run>/staging/chrome-cmd.txt` 写 `stop` / `start` / `report` / `enumerate`（1s 内生效）；`none` = 空闲。
- 报告流：`<run>/logs/chrome-reports.jsonl`（每条含 state/device/settings/frameCount/fps/events/errors）。

## 判据

- 独立消费 PASS：`state=RUNNING`、`frameCount` 持续增长、`FPS(rolling30)≈30`、截图显示合成画面（行内 std=0 判别同 M0-T2 `analyze-preview.ps1`）。
- 移除设备：`mute`/`ended` 事件时间戳 + FPS 归零/报错如实记录；重注册后需手动 `start` 才恢复（预期，实测确认）。
