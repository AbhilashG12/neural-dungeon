use crate::app::{GameApp, UiMode};
use game_core::world::Tile;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};

const COLOR_NEON_CYAN: Color = Color::Rgb(0, 255, 255);
const COLOR_NEON_MAGENTA: Color = Color::Rgb(255, 0, 255);
const COLOR_TECH_GRAY: Color = Color::Rgb(50, 50, 70);
const COLOR_HUD_GREEN: Color = Color::Rgb(0, 255, 128);

pub fn render(f: &mut Frame, app: &GameApp) {
    let area = f.size();

    // 75% Map Viewport (Left) | 25% Telemetry/HUD Sidebar (Right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(area);

    let sidebar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),  // Stats & Level
            Constraint::Min(14),    // Directive & Info
            Constraint::Length(10), // Combat Log
        ])
        .split(main_chunks[1]);

    // Render core layout components
    render_map(f, app, main_chunks[0]);
    render_stats(f, app, sidebar_chunks[0]);
    render_info(f, app, sidebar_chunks[1]);
    render_log(f, app, sidebar_chunks[2]);

    // Render active modal overlays
    match &app.ui_mode {
        UiMode::Paused => render_pause_menu(f, area),
        UiMode::Inventory => render_inventory_menu(f, app, area),
        UiMode::GameOver => render_game_over_menu(f, area),
        UiMode::Victory => render_victory_menu(f, area),
        UiMode::Help | UiMode::HelpMenu => render_help_menu(f, area),
        UiMode::Dialogue {
            npc_id,
            input_buffer,
            is_waiting,
        } => render_dialogue_menu(f, app, *npc_id, input_buffer, *is_waiting, area),
        UiMode::Puzzle { puzzle, state, .. } => render_puzzle_menu(f, puzzle, state, area),
        UiMode::PuzzleSolved { explanation } => render_puzzle_solved_menu(f, explanation, area),
        UiMode::Playing => {}
    }
}

fn get_primary_color(theme: &crate::app::Theme) -> Color {
    match theme {
        crate::app::Theme::Cyberpunk => Color::Rgb(0, 255, 255),
        crate::app::Theme::Monochrome => Color::White,
    }
}

fn create_sci_fi_block(title: &str) -> Block<'static> {
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(COLOR_NEON_CYAN))
        .title_style(
            Style::default()
                .fg(COLOR_NEON_CYAN)
                .add_modifier(Modifier::BOLD),
        )
}

fn render_map(f: &mut Frame, app: &GameApp, area: Rect) {
    let block = create_sci_fi_block(&format!(
        "SECTOR 7: LABORATORY // DEPTH: {}",
        app.state.floor_number
    ));
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let floor = &app.state.current_floor;
    let (px, py) = app.state.player.position;

    let map_char_width = (floor.width as u16) * 2;
    let map_char_height = floor.height as u16;

    let pad_left = inner_area.width.saturating_sub(map_char_width) / 2;
    let pad_top = inner_area.height.saturating_sub(map_char_height) / 2;

    let mut map_lines = Vec::new();

    // Top vertical padding to center the static board
    for _ in 0..pad_top {
        map_lines.push(Line::from(""));
    }

    let left_margin = " ".repeat(pad_left as usize);

    for y in 0..floor.height as i32 {
        if map_lines.len() >= inner_area.height as usize {
            break;
        }

        let mut row = Vec::with_capacity(floor.width + 1);
        if pad_left > 0 {
            row.push(Span::raw(left_margin.clone()));
        }

        for x in 0..floor.width as i32 {
            if (x as u16 * 2) + pad_left + 2 > inner_area.width {
                break;
            }

            if !floor.is_explored(x, y) {
                row.push(Span::raw("  "));
                continue;
            }

            if (px, py) == (x, y) {
                if app.state.player.hit_timer > 0 {
                    row.push(Span::styled(
                        "@ ",
                        Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD),
                    ));
                } else {
                    let color = if app.state.player.evading {
                        COLOR_NEON_MAGENTA
                    } else {
                        COLOR_NEON_CYAN
                    };
                    row.push(Span::styled(
                        "@ ",
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ));
                }
            } else if let Some(_npc) = floor.get_npc_at(x, y) {
                row.push(Span::styled(
                    "N ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            } else if let Some(enemy) = floor.get_enemy_at(x, y) {
                let glyph = if enemy.id == 9999 { "M " } else { "Ω " };
                
                if enemy.hit_timer > 0 {
                    row.push(Span::styled(
                        glyph,
                        Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD),
                    ));
                } else {
                    let is_telegraphing = matches!(
                        enemy.state,
                        game_core::world::EnemyState::Telegraphing { .. }
                    );
                    let color = if is_telegraphing { Color::Yellow } else { Color::Red };
                    row.push(Span::styled(
                        glyph,
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ));
                }
            } else if let Some(item) = floor.items.iter().find(|i| i.position == (x, y)) {
                match &item.item_type {
                    game_core::world::ItemType::Key { .. } => {
                        row.push(Span::styled(
                            "✦ ",
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        ));
                    }
                    game_core::world::ItemType::Consumable { .. } => {
                        row.push(Span::styled(
                            "+ ",
                            Style::default()
                                .fg(COLOR_HUD_GREEN)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                    game_core::world::ItemType::Weapon { .. } => {
                        row.push(Span::styled(
                            "W ",
                            Style::default()
                                .fg(COLOR_NEON_MAGENTA)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                }
            } else {
                let symbol = match floor.get_tile(x, y).unwrap_or(Tile::Wall) {
                    Tile::Wall => Span::styled("██", Style::default().fg(COLOR_TECH_GRAY)),
                    Tile::Floor => Span::styled("· ", Style::default().fg(Color::DarkGray)),
                    Tile::Door { is_open: true, .. } => Span::styled("/ ", Style::default().fg(Color::Green)),
                    Tile::Door { is_open: false, .. } => Span::styled("▣ ", Style::default().fg(Color::Yellow)),
                    Tile::StairsDown => Span::styled("▼ ", Style::default().fg(COLOR_NEON_MAGENTA)),
                    Tile::Hazard { .. } => {
                        let hazard_color = if app.theme == crate::app::Theme::Cyberpunk {
                            Color::Red
                        } else {
                            Color::DarkGray
                        };
                        Span::styled("▲ ", Style::default().fg(hazard_color))
                    }
                    Tile::Terminal { is_solved: true, .. } => Span::styled(
                        "T ",
                        Style::default().fg(COLOR_HUD_GREEN).add_modifier(Modifier::BOLD),
                    ),
                    Tile::Terminal { is_solved: false, .. } => Span::styled(
                        "T ",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                };
                row.push(symbol);
            }
        }
        map_lines.push(Line::from(row));
    }

    let map_widget = Paragraph::new(map_lines);
    f.render_widget(map_widget, inner_area);
}

fn render_stats(f: &mut Frame, app: &GameApp, area: Rect) {
    let p = &app.state.player;
    let block = create_sci_fi_block("NEURAL LINK");
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner_area);

    let center_80_percent = |r: Rect| -> Rect {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(10),
                Constraint::Percentage(80),
                Constraint::Percentage(10),
            ])
            .split(r)[1]
    };

    let level_text = Paragraph::new(format!(
        "LEVEL: {} | XP: {}/{}",
        p.level,
        p.xp,
        p.level * 150
    ))
    .style(Style::default().fg(get_primary_color(&app.theme)))
    .alignment(Alignment::Center);
    f.render_widget(level_text, vertical_chunks[0]);

    let hp_ratio = (p.hp as f64 / p.max_hp as f64).clamp(0.0, 1.0);
    let hp_gauge = Gauge::default()
        .block(Block::default().title("INTEGRITY (HP)"))
        .gauge_style(Style::default().fg(COLOR_HUD_GREEN).bg(COLOR_TECH_GRAY))
        .ratio(hp_ratio);

    let energy_ratio = (p.energy as f64 / 100.0).clamp(0.0, 1.0);
    let energy_gauge = Gauge::default()
        .block(Block::default().title("SYSTEM POWER"))
        .gauge_style(Style::default().fg(COLOR_NEON_CYAN).bg(COLOR_TECH_GRAY))
        .ratio(energy_ratio);

    f.render_widget(hp_gauge, center_80_percent(vertical_chunks[1]));
    f.render_widget(energy_gauge, center_80_percent(vertical_chunks[2]));

    let weapon_name = if let Some(w) = &p.weapon {
        w.name.clone()
    } else {
        "Standard Core".to_string()
    };
    let weapon_text = Paragraph::new(Span::styled(
        format!("WEAPON: {}", weapon_name),
        Style::default().fg(Color::DarkGray),
    ))
    .alignment(Alignment::Center);
    f.render_widget(weapon_text, vertical_chunks[3]);
}

fn render_info(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = vec![];

    let (threat_label, threat_color) = match app.state.floor_number {
        1 => ("MINIMAL", COLOR_HUD_GREEN),
        2 | 3 => ("ELEVATED", Color::Yellow),
        4 => ("SEVERE", Color::LightRed),
        5.. => ("CRITICAL (CORE)", Color::Red),
        _ => ("UNKNOWN", Color::DarkGray),
    };

    text.push(Line::from(Span::styled(
        " ENVIRONMENT METRICS:",
        Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD),
    )));
    text.push(Line::from(format!("  SECTOR DEPTH : {} / 5", app.state.floor_number)));
    text.push(Line::from(vec![
        Span::raw("  THREAT LEVEL : "),
        Span::styled(threat_label, Style::default().fg(threat_color).add_modifier(Modifier::BOLD)),
    ]));
    text.push(Line::from("  LLM ENGINE   : Q4_K_M (ACTIVE)"));

    text.push(Line::from(""));

    text.push(Line::from(Span::styled(
        " ACTIVE DIRECTIVE:",
        Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD),
    )));
    for quest in &app.state.player.active_quests {
        let (icon, color) = if quest.completed {
            ("[x]", COLOR_HUD_GREEN)
        } else {
            ("[ ]", Color::Yellow)
        };
        text.push(Line::from(Span::styled(
            format!("  {} {}", icon, quest.title),
            Style::default().fg(color),
        )));
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "  [?, H] OPEN SYSTEM MANUAL",
        Style::default().fg(COLOR_NEON_MAGENTA).add_modifier(Modifier::RAPID_BLINK),
    )));

    let block = Paragraph::new(text)
        .block(create_sci_fi_block("DATABANKS"))
        .wrap(Wrap { trim: true });

    f.render_widget(block, area);
}

fn render_log(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = Vec::new();
    let display_count = area.height.saturating_sub(2) as usize;
    let start_idx = app.log_messages.len().saturating_sub(display_count);

    let messages = &app.log_messages[start_idx..];
    
    for (i, msg) in messages.iter().enumerate() {
        let is_latest = i == messages.len().saturating_sub(1);
        
        let style = if is_latest {
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        
        text.push(Line::from(Span::styled(format!("> {}", msg), style)));
    }

    let block = Paragraph::new(text)
        .block(create_sci_fi_block("TELEMETRY LOG"))
        .wrap(Wrap { trim: true });

    f.render_widget(block, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_pause_menu(f: &mut Frame, area: Rect) {
    let block = Paragraph::new("\n\n[ESC] Resume Protocol\n[Q] Terminate Session")
        .block(
            Block::default()
                .title(" SYSTEM HALTED ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow).bg(Color::Black));
    let popup_area = centered_rect(30, 20, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
}

fn render_inventory_menu(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = Vec::new();
    if app.state.player.inventory.is_empty() {
        text.push(Line::from("Storage banks empty."));
    } else {
        for (i, item) in app.state.player.inventory.iter().enumerate() {
            text.push(Line::from(format!("[{}] {}", i, item.name)));
        }
        text.push(Line::from(""));
        text.push(Line::from(Span::styled(
            "[U] Consume/Equip top item   |   [C] Craft / Combine items",
            Style::default().fg(Color::Yellow),
        )));
    }
    let block = Paragraph::new(text).block(
        Block::default()
            .title(" CACHE ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_NEON_MAGENTA)),
    );
    let popup_area = centered_rect(50, 40, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
}

fn render_game_over_menu(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from("[R] Initialize New Clone"),
        Line::from("[Q] Terminate"),
    ];
    let block = Paragraph::new(text)
        .block(
            Block::default()
                .title(" CRITICAL SYSTEM FAILURE ")
                .borders(Borders::ALL)
                .border_type(BorderType::Thick),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red).bg(Color::Black));
    let popup_area = centered_rect(30, 20, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
}

fn render_dialogue_menu(
    f: &mut Frame,
    app: &GameApp,
    npc_id: usize,
    input: &str,
    is_waiting: bool,
    area: Rect,
) {
    let npc = app
        .state
        .current_floor
        .npcs
        .iter()
        .find(|n| n.id == npc_id)
        .unwrap();

    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Length(3)])
        .split(popup_area);

    let mut history_text = Vec::new();
    for msg in &npc.memory {
        if msg.starts_with("Player:") {
            history_text.push(Line::from(Span::styled(
                msg,
                Style::default().fg(COLOR_NEON_CYAN),
            )));
        } else {
            history_text.push(Line::from(Span::styled(
                msg,
                Style::default().fg(Color::Yellow),
            )));
        }
    }

    if history_text.is_empty() {
        history_text.push(Line::from(Span::styled(
            "Neural handshake established...",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let history_block = Paragraph::new(history_text)
        .block(create_sci_fi_block(&npc.name).border_style(Style::default().fg(Color::Yellow)))
        .wrap(Wrap { trim: true });
    f.render_widget(history_block, chunks[0]);

    let input_text = if is_waiting {
        Span::styled(
            "[ PROCESSING NEURAL INFERENCE... ]",
            Style::default()
                .fg(COLOR_NEON_MAGENTA)
                .add_modifier(Modifier::RAPID_BLINK),
        )
    } else {
        Span::raw(format!("> {}_", input))
    };

    let input_block = Paragraph::new(input_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(COLOR_NEON_CYAN)),
    );
    f.render_widget(input_block, chunks[1]);
}

fn render_help_menu(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(Span::styled(
            "NEURAL DUNGEON OPERATING MANUAL",
            Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("TACTICAL LEGEND", Style::default().fg(Color::Yellow))),
        Line::from("  @ Player     Ω Hostile / M Nexus Mind"),
        Line::from("  ✦ Loot/Key   ▣ Locked / / Open Door"),
        Line::from("  ▼ Descent    · Floor / ██ Wall"),
        Line::from("  N NPC        T Puzzle Terminal"),
        Line::from("  ▲ Hazard"),
        Line::from(""),
        Line::from(Span::styled("MOVEMENT & COMBAT", Style::default().fg(Color::Yellow))),
        Line::from("[W, A, S, D] : Navigate grid. Walk into enemies to strike."),
        Line::from("[X]          : Execute a dedicated standard strike (adjacent target)."),
        Line::from("[Q]          : Heavy Overload Strike (200% dmg, costs 140 energy)."),
        Line::from("[E]          : Evade & Riposte. Use when enemy flashes yellow."),
        Line::from("[Space]      : Wait one time-tick to recharge energy."),
        Line::from(""),
        Line::from(Span::styled("WORLD INTERACTION", Style::default().fg(Color::Yellow))),
        Line::from("[F]          : Interact. Open doors, talk to NPCs, access Terminals."),
        Line::from("[G]          : Grab item. Stand over +/W/✦ to extract to inventory."),
        Line::from("[>]          : Descend stairs. Must be standing on ▼."),
        Line::from(""),
        Line::from(Span::styled("SYSTEM PROTOCOLS", Style::default().fg(Color::Yellow))),
        Line::from("[I]          : Open Inventory Cache."),
        Line::from("[U]          : Consume/Equip top item in inventory."),
        Line::from("[C]          : Craft/Combine 2x Health Patches (Inside Inventory)."),
        Line::from("[T]          : Toggle Display Theme (Cyberpunk / Monochrome)."),
        Line::from("[ESC]        : Pause system operations or close menus."),
        Line::from(""),
        Line::from(Span::styled("Press [ESC] to close manual.", Style::default().fg(Color::DarkGray))),
    ];

    let block = Paragraph::new(text)
        .block(
            Block::default()
                .title(" SYSTEM MANUAL ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(COLOR_NEON_CYAN)),
        )
        .alignment(Alignment::Left);

    let popup_area = centered_rect(75, 90, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
}

fn render_puzzle_menu(
    f: &mut Frame,
    puzzle: &game_core::world::PuzzleType,
    state: &game_core::puzzle::PuzzleState,
    area: Rect,
) {
    let popup_area = centered_rect(68, 72, area);
    f.render_widget(Clear, popup_area);

    let mut text = Vec::new();

    // Extract the active_feedback cleanly without mutating a dummy variable
    let active_feedback: &Option<String> = match state {
        game_core::puzzle::PuzzleState::TokenVault { feedback, .. } => feedback,
        game_core::puzzle::PuzzleState::QuantizationForge { feedback, .. } => feedback,
        game_core::puzzle::PuzzleState::ContextCollapse { feedback, .. } => feedback,
        game_core::puzzle::PuzzleState::InferenceReactor { feedback, .. } => feedback,
    };

    match state {
        game_core::puzzle::PuzzleState::TokenVault {
            target,
            prompt_hint,
            fragments,
            selected,
            expected_len,
            ..
        } => {
            text.push(Line::from(Span::styled(
                format!("OBJECTIVE: {}", prompt_hint),
                Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from(format!(
                "TARGET STRING: '{}'  |  REQUIRED SUB-WORDS: {}",
                target, expected_len
            )));
            text.push(Line::from(""));
            text.push(Line::from(
                "VOCABULARY FRAGMENTS (Press 0-9 in exact reading order, 'C' to clear):",
            ));
            for (i, frag) in fragments.iter().enumerate() {
                let order_badge = if let Some(pos) = selected.iter().position(|&x| x == i) {
                    format!("[#{}]", pos + 1)
                } else {
                    "[  ]".to_string()
                };
                let style = if selected.contains(&i) {
                    Style::default().fg(COLOR_HUD_GREEN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                text.push(Line::from(Span::styled(
                    format!("  {} Key [{}] -> '{}' (Token ID: {})", order_badge, i, frag.0, frag.1),
                    style,
                )));
            }
            let current: String = selected.iter().map(|&i| fragments[i].0).collect();
            let token_ids: Vec<String> = selected.iter().map(|&i| fragments[i].1.to_string()).collect();
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                format!("ASSEMBLED TEXT : '{}'", current),
                Style::default().fg(Color::Yellow),
            )));
            text.push(Line::from(Span::styled(
                format!("TOKEN ID TENSOR: [{}] ({}/{} tokens)", token_ids.join(", "), selected.len(), expected_len),
                Style::default().fg(COLOR_NEON_CYAN),
            )));
        }
        game_core::puzzle::PuzzleState::QuantizationForge {
            model_params_b,
            context_kv_gb,
            vram_limit_gb,
            min_coherence,
            current_bits,
            ..
        } => {
            let weight_vram = (*model_params_b as f32) * (*current_bits as f32 / 8.0);
            let total_vram = weight_vram + *context_kv_gb;
            let coherence = match *current_bits {
                32 => 100,
                16 => 99,
                8 => 98,
                4 => 95,
                _ => 45,
            };

            text.push(Line::from(Span::styled(
                "OBJECTIVE: Configure weight quantization to fit hardware VRAM without collapsing coherence.",
                Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from("Use [W/S] or [UP/DOWN] to step precision between FP32, FP16, INT8, INT4, INT2."));
            text.push(Line::from(""));
            text.push(Line::from(format!(
                "MODEL ARCHITECTURE : {} Billion Parameters  |  KV CACHE OVERHEAD: {:.1} GB",
                model_params_b, context_kv_gb
            )));
            text.push(Line::from(Span::styled(
                format!(
                    "HARDWARE CONSTRAINTS: Max VRAM <= {:.1} GB  |  Min Coherence >= {}%",
                    vram_limit_gb, min_coherence
                ),
                Style::default().fg(Color::Yellow),
            )));
            text.push(Line::from(""));
            text.push(Line::from(format!(
                "ACTIVE PRECISION   : {}-bit ({:.2} bytes/param)",
                current_bits,
                *current_bits as f32 / 8.0
            )));

            let vram_color = if total_vram <= *vram_limit_gb { COLOR_HUD_GREEN } else { Color::Red };
            let coh_color = if coherence >= *min_coherence { COLOR_HUD_GREEN } else { Color::Red };

            text.push(Line::from(Span::styled(
                format!(
                    "PROJECTED VRAM     : {:.1} GB Weights + {:.1} GB KV = {:.1} / {:.1} GB",
                    weight_vram, context_kv_gb, total_vram, vram_limit_gb
                ),
                Style::default().fg(vram_color).add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from(Span::styled(
                format!("MODEL COHERENCE    : {}% (Required: >= {}%)", coherence, min_coherence),
                Style::default().fg(coh_color).add_modifier(Modifier::BOLD),
            )));
        }
        game_core::puzzle::PuzzleState::ContextCollapse {
            system_goal,
            logs,
            selected,
            max_budget,
            ..
        } => {
            text.push(Line::from(Span::styled(
                format!("OBJECTIVE: Prune KV Cache for task: '{}'", system_goal),
                Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from(format!(
                "Keep essential specs, remove noise & prompt-injection traps, and stay <= {} Tokens.",
                max_budget
            )));
            text.push(Line::from(""));
            let mut current_cost = 0;
            for (i, log) in logs.iter().enumerate() {
                let prefix = if selected.contains(&i) {
                    current_cost += log.1;
                    "[KEEP]"
                } else {
                    "[DROP]"
                };
                let style = if selected.contains(&i) {
                    Style::default().fg(Color::White)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                text.push(Line::from(Span::styled(
                    format!("  {} Key [{}]: {} ({} Tokens)", prefix, i, log.0, log.1),
                    style,
                )));
            }
            let color = if current_cost > *max_budget { Color::Red } else { COLOR_HUD_GREEN };
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                format!("ACTIVE KV CACHE USAGE: {} / {} Tokens", current_cost, max_budget),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )));
        }
        game_core::puzzle::PuzzleState::InferenceReactor {
            scenario_name,
            stages,
            cursor,
            selected,
            ..
        } => {
            text.push(Line::from(Span::styled(
                format!("OBJECTIVE: Reconstruct Pipeline -> {}", scenario_name),
                Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from("Use [W/S] or [UP/DOWN] to move cursor. Press [SPACE] to select & swap stages."));
            text.push(Line::from(""));
            for (i, stage) in stages.iter().enumerate() {
                let is_cursor = *cursor == i;
                let is_selected = *selected == Some(i);

                let style = if is_selected {
                    Style::default().fg(COLOR_NEON_MAGENTA).add_modifier(Modifier::BOLD)
                } else if is_cursor {
                    Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(Color::White)
                };

                text.push(Line::from(Span::styled(format!("  Step {}: {}  ", i + 1, stage), style)));
            }
        }
    }

    text.push(Line::from(""));
    if let Some(err_msg) = active_feedback {
        text.push(Line::from(Span::styled(
            format!(">> DIAGNOSTIC: {}", err_msg),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    } else {
        text.push(Line::from(Span::styled(
            ">> STATUS: Ready to compile. Press [ENTER] to test configuration.",
            Style::default().fg(Color::Yellow),
        )));
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "[ENTER] Compile & Verify   [C/Backspace] Clear Buffer   [ESC] Abort",
        Style::default().fg(Color::DarkGray),
    )));

    let title = format!(" TERMINAL // {:?} ", puzzle);
    let block = Paragraph::new(text)
        .block(create_sci_fi_block(&title))
        .wrap(Wrap { trim: true });
    f.render_widget(block, popup_area);
}

fn render_puzzle_solved_menu(f: &mut Frame, explanation: &str, area: Rect) {
    let popup_area = centered_rect(60, 40, area);
    f.render_widget(Clear, popup_area);

    let mut text = vec![
        Line::from(Span::styled(
            " NEURAL LINK ESTABLISHED ",
            Style::default()
                .fg(COLOR_HUD_GREEN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    text.push(Line::from(Span::styled(
        explanation,
        Style::default().fg(Color::White),
    )));

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "[ESC] Acknowledge & Continue",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Paragraph::new(text)
        .block(
            create_sci_fi_block("DATA LOG EXTRACTED")
                .border_style(Style::default().fg(COLOR_HUD_GREEN)),
        )
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);

    f.render_widget(block, popup_area);
}

fn render_victory_menu(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from("SECTOR 7 SECURED"),
        Line::from("The facility is under your control."),
        Line::from(""),
        Line::from("[R] Initialize New Run"),
        Line::from("[Q] Terminate Connection"),
    ];
    
    let block = Paragraph::new(text)
        .block(
            Block::default()
                .title(" ESCAPE VELOCITY ACHIEVED ")
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(Style::default().fg(COLOR_HUD_GREEN)),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(COLOR_HUD_GREEN).bg(Color::Black));
        
    let popup_area = centered_rect(40, 25, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
}
