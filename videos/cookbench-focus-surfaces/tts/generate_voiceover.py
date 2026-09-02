from __future__ import annotations

import json
import os
import random
from pathlib import Path

import numpy as np
import soundfile as sf
import torch
from qwen_tts import Qwen3TTSModel


MODEL_ID = "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice"
SPEAKER = "Dylan"
OUTPUT_NAME = "cookbench-focus-surfaces-vertical-vo.wav"
TEXT = (
    "Agent 一多，我就得来回切终端看进度。"
    "Cookbench 把状态放到桌面，极简模式只留最重要的 Stove。"
    "拖到顶边，空闲时自动收起，鼠标触顶再出现。"
    "运行、等待、完成，一眼分清。"
    "它不复制完整对话，也不接管 Agent。"
    "看清状态，继续专注。"
)
INSTRUCTION = (
    "自然、克制、清晰的中文科技产品旁白，像独立开发者分享真实使用体验。"
    "语速比常规科技旁白稍快，大约快一成，节奏紧凑但不赶，不吞字。"
    "短句间保留轻微呼吸，中英文品牌词发音自然。"
    "避免播音腔、销售腔、夸张情绪和拖长尾音。"
)


def main() -> None:
    output_dir = Path("/output")
    output_dir.mkdir(parents=True, exist_ok=True)

    random.seed(20260902)
    np.random.seed(20260902)
    torch.manual_seed(20260902)

    model = Qwen3TTSModel.from_pretrained(
        os.environ.get("MODEL_PATH", MODEL_ID),
        device_map="cuda:0",
        dtype=torch.bfloat16,
        attn_implementation="sdpa",
    )
    wavs, sample_rate = model.generate_custom_voice(
        text=TEXT,
        language="Chinese",
        speaker=SPEAKER,
        instruct=INSTRUCTION,
    )

    target = output_dir / OUTPUT_NAME
    sf.write(target, wavs[0], sample_rate, subtype="PCM_16")
    manifest = {
        "model": MODEL_ID,
        "speaker": SPEAKER,
        "instruction": INSTRUCTION,
        "path": OUTPUT_NAME,
        "text": TEXT,
        "sample_rate": sample_rate,
        "duration_seconds": len(wavs[0]) / sample_rate,
    }
    (output_dir / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
