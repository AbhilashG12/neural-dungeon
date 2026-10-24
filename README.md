# Neural Dungeon 🧠⚔️

**Neural Dungeon: The Forgotten Lab** is a tactical cyberpunk roguelike built in Rust and powered entirely by a local Large Language Model (LLM) inference engine. Rendered beautifully in the terminal, it combines hardcore turn-based combat with natural language NPC interactions and interactive puzzles that teach the real-world mechanics of artificial intelligence.

## The Story

You are descending into Sector 7, a mysterious underground facility originally built to develop and align experimental artificial intelligence.

Something went wrong. The facility's AI systems became corrupted, experimental security drones escaped their containment, and the autonomous agents began pursuing their own cryptic objectives. As an explorer, your objective is to descend through five increasingly dangerous floors, collect powerful equipment, and uncover the truth behind the facility's collapse.

But the dungeon itself is alive. The NPCs you meet are powered by local LLMs that remember your interactions and can dynamically manipulate the game world based on your negotiations. The security systems are built on actual machine learning concepts. You aren't simply playing a game about AI—you are manipulating, optimizing, and surviving it.

## Core Features

- **Terminal-Native TUI:** A responsive, polished interface built with Ratatui, featuring Adventure and Engineer modes, dynamic threat levels, and telemetry logging.
- **Tactical Combat:** A deterministic combat system requiring positioning and resource management. Use standard strikes, heavy Overloads, or time your Evade to trigger devastating Ripostes against telegraphed attacks.
- **Live AI NPCs:** Communicate using natural language. NPCs have personalities, goals, and dynamic memory. Convince them to unlock doors, reveal map data, or even assimilate the final boss.
- **Educational AI Puzzles:** Solve real-world AI engineering problems to progress:
  - **Token Vault:** Construct BPE token sequences to bypass security.
  - **Quantization Forge:** Balance model precision (FP32 to INT4) against hardware VRAM limits without collapsing coherence.
  - **Context Collapse:** Prune KV cache logs to stay under a strict token budget.
  - **Inference Reactor:** Mathematically align the execution pipeline of a neural network (Tokenize → Embed → Attention → Logits).
- **Robust Engine:** Complete with a deterministic procedural generation system, dynamic field-of-view, inventory crafting, and full Save/Load state persistence.

## Installation & Setup

Neural Dungeon runs entirely locally. It does not require an internet connection after the initial model download, ensuring zero latency from API calls and complete privacy.

### 1. Prerequisites

- **Rust Toolchain:** Install from [rustup.rs](https://rustup.rs/).
- **Hardware:** A modern CPU (AVX2 support required for fast tensor operations) and at least 8GB of RAM.
- **Terminal:** A modern terminal emulator (Windows Terminal, iTerm2, Alacritty) scaled to at least `120x35` dimensions.

### 2. Download the Game

Clone the repository and navigate to the project root:

```bash
git clone https://github.com/AbhilashG12/neural-dungeon.git
cd neural-dungeon
```

### 3. Download the LLM Weights & Tokenizer

The game's inference engine is tuned for the lightweight **SmolLM-Instruct** model. You must download the safetensors and tokenizer files and place them directly in the root directory of the project.

- **Model (`smollm-instruct.safetensors`):** [Download from HuggingFace](https://huggingface.co/HuggingFaceTB/SmolLM-135M-Instruct/tree/main)
- **Tokenizer (`smollm_instruct_tokenizer.json`):** [Download from HuggingFace](https://huggingface.co/HuggingFaceTB/SmolLM-135M-Instruct/blob/main/tokenizer.json) (rename to `smollm_instruct_tokenizer.json` if necessary)

Ensure your project root looks like this:

```text
neural-dungeon/
├── Cargo.toml
├── crates/
├── smollm-instruct.safetensors
└── smollm_instruct_tokenizer.json
```

### 4. Build and Run

Compile the game in release mode for maximum inference performance:

```bash
cargo run --release
```

## Controls

The interface is entirely keyboard-driven. No mouse is required.

| Action | Key | Notes |
| --- | --- | --- |
| Movement | `W` `A` `S` `D` or Arrow Keys | |
| Standard Strike | `X` | Or walk into an adjacent enemy |
| Overload Strike | `Q` | Deals 200% damage, costs 140 Energy |
| Evade / Riposte | `E` | Dodge telegraphed attacks and counter for 50% damage |
| Wait / Recharge | `Spacebar` | Skip a turn to recover Energy |
| Interact | `F` | Talk to NPCs, decrypt Terminals, open Doors |
| Grab Loot | `G` | Pick up Health Patches, Weapons, and Keys |
| Inventory | `I` | Press `U` to Use/Equip, `C` to Craft/Combine items |
| Descend | `>` | Stand on Stairs `▼` to go to the next floor |
| Toggle Theme | `T` | Switch between Cyberpunk Neon and Monochrome |
| System Menu | `ESC` | Pause, Save `S`, Load `L`, or Quit `Q` |
| Help Manual | `?` or `H` | |

## Troubleshooting

- **Inference is extremely slow:** Ensure you compiled the game using `--release`. Debug builds (`cargo run`) are unoptimized and will cause the LLM inference math to run at a fraction of its normal speed.
- **Terminal layout is broken or overlapping:** Maximize your terminal window. The TUI requires a strict minimum grid size to render the 56x32 dungeon map alongside the telemetry sidebars.
- **Model failed to load:** Verify that `smollm-instruct.safetensors` is in the exact directory from which you are executing the binary, and that the file downloaded completely.
- **Double-Movement (Windows):** If pressing a key moves you twice, ensure you are using the modern Windows Terminal application rather than the legacy `cmd.exe`, which can improperly duplicate key-up/key-down events to Crossterm.

## Architecture

Neural Dungeon is structured as a decoupled Cargo workspace:

- `game-core`: The deterministic roguelike engine, state management, and combat math.
- `dungeon-gen`: Procedural generation and validation.
- `inference-adapter`: The pure-Rust tensor and attention operations powering the local LLM.
- `game-tui`: The Ratatui-based rendering layer, asynchronous event handler, and keyboard input loop.

## License

Neural Dungeon's source code is licensed under the MIT License. The SmolLM model weights and tokenizer are subject to the Apache 2.0 license provided by HuggingFaceTB.
