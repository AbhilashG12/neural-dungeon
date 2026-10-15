use crate::app::{GameApp, UiMode};
use game_core::world::Tile;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Wrap},
};

const COLOR_NEON_CYAN: Color = Color::Rgb(0, 255, 255);
const COLOR_NEON_MAGENTA: Color = Color::Rgb(255, 0, 255);
const COLOR_TECH_GRAY: Color = Color::Rgb(50, 50, 70);
const COLOR_HUD_GREEN: Color = Color::Rgb(0, 255, 128);

pub fn render(f: &mut Frame, app: &GameApp) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(f.size());

    let sidebar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),
            Constraint::Min(14),
            Constraint::Length(10),
        ])
        .split(main_chunks[1]);

    render_map(f, app, main_chunks[0]);
    render_stats(f, app, sidebar_chunks[0]);
    render_info(f, app, sidebar_chunks[1]);
    render_log(f, app, sidebar_chunks[2]);

    match &app.ui_mode {
        UiMode::Paused => render_pause_menu(f, f.size()),
        UiMode::Inventory => render_inventory_menu(f, app, f.size()),
        UiMode::GameOver => render_game_over_menu(f, f.size()),
        UiMode::Help => render_help_menu(f, f.size()),
        UiMode::Dialogue {
            npc_id,
            input_buffer,
            is_waiting,
        } => render_dialogue_menu(f, app, *npc_id, input_buffer, *is_waiting, f.size()),
        UiMode::Puzzle { puzzle, state, .. } => render_puzzle_menu(f, puzzle, state, f.size()),
        UiMode::PuzzleSolved { explanation } => render_puzzle_solved_menu(f, explanation, f.size()),
        UiMode::HelpMenu => render_help_menu(f, f.size()),
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
    let floor = &app.state.current_floor;
    let mut map_lines = Vec::new();

    for y in 0..floor.height as i32 {
        let mut row = Vec::new();
        for x in 0..floor.width as i32 {
            if !floor.is_explored(x, y) {
                row.push(Span::raw(" "));
                continue;
            }

            if app.state.player.position == (x, y) {
                let color = if app.state.player.evading {
                    COLOR_NEON_MAGENTA
                } else {
                    COLOR_NEON_CYAN
                };
                row.push(Span::styled(
                    "@",
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
            } else if let Some(npc) = floor.get_npc_at(x, y) {
                row.push(Span::styled(
                    npc.symbol.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            } else if let Some(enemy) = floor.get_enemy_at(x, y) {
                if enemy.id == 9999 {
                    // THE NEXUS MIND (Boss Sprite)
                    let style = if matches!(
                        enemy.state,
                        game_core::world::EnemyState::Telegraphing { .. }
                    ) {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK)
                    } else {
                        Style::default()
                            .fg(Color::Red)
                            .add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK)
                    };
                    row.push(Span::styled("M", style)); // 'M' for Mind
                } else if matches!(
                    enemy.state,
                    game_core::world::EnemyState::Telegraphing { .. }
                ) {
                    row.push(Span::styled(
                        "Ω",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else {
                    row.push(Span::styled(
                        "Ω",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ));
                }
            } else if let Some(item) = floor.items.iter().find(|i| i.position == (x, y)) {
                // FIXED: Added Weapon arm to satisfy exhaustive pattern matching
                match item.item_type {
                    game_core::world::ItemType::Key { .. } => {
                        row.push(Span::styled(
                            "K",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                    game_core::world::ItemType::Consumable { .. } => {
                        row.push(Span::styled(
                            "+",
                            Style::default()
                                .fg(COLOR_HUD_GREEN)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                    game_core::world::ItemType::Weapon { .. } => {
                        row.push(Span::styled(
                            "W",
                            Style::default()
                                .fg(COLOR_NEON_MAGENTA)
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                }
            } else {
                match floor.get_tile(x, y).unwrap_or(Tile::Wall) {
                    Tile::Wall => row.push(Span::styled("█", Style::default().fg(COLOR_TECH_GRAY))),
                    Tile::Floor => {
                        row.push(Span::styled("·", Style::default().fg(Color::DarkGray)))
                    }
                    Tile::Door { is_open, .. } => {
                        if is_open {
                            row.push(Span::styled("·", Style::default().fg(Color::DarkGray)));
                        } else {
                            row.push(Span::styled("▣", Style::default().fg(Color::Yellow)));
                        }
                    }
                    Tile::StairsDown => {
                        row.push(Span::styled("▼", Style::default().fg(COLOR_NEON_MAGENTA)))
                    }
                    Tile::Hazard { .. } => {
                        let hazard_color = if app.theme == crate::app::Theme::Cyberpunk {
                            Color::Red
                        } else {
                            Color::DarkGray
                        };
                        row.push(Span::styled("▲", Style::default().fg(hazard_color)));
                    }
                    Tile::Terminal { is_solved, .. } => {
                        let color = if is_solved {
                            COLOR_HUD_GREEN
                        } else {
                            Color::Yellow
                        };
                        row.push(Span::styled(
                            "T",
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ));
                    }
                }
            }
        }
        map_lines.push(Line::from(row));
    }

    let map_block = Paragraph::new(map_lines).block(create_sci_fi_block("SECTOR 7: LABORATORY"));
    f.render_widget(map_block, area);
}

fn render_stats(f: &mut Frame, app: &GameApp, area: Rect) {
    let p = &app.state.player;
    let block = create_sci_fi_block("NEURAL LINK");
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Fix 1: Properly define the constraints for all 4 elements
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Index 0: Level & XP Text
            Constraint::Length(2), // Index 1: HP Gauge
            Constraint::Length(2), // Index 2: Energy Gauge
            Constraint::Length(1), // Index 3: Weapon Text
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

    // Fix 2: Render Level/XP using vertical_chunks[0] instead of the undefined 'chunks[2]'
    let level_text = Paragraph::new(format!(
        "LEVEL: {} | XP: {}/{}",
        p.level,
        p.xp,
        p.level * 150
    ))
    .style(Style::default().fg(get_primary_color(&app.theme)))
    .alignment(ratatui::layout::Alignment::Center); // Centered looks better above gauges
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

    let weapon_text = Paragraph::new(Span::styled(
        "WEAPON: Plasma Blade",
        Style::default().fg(Color::DarkGray),
    ))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(weapon_text, vertical_chunks[3]);
}

fn render_info(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = vec![];

    // Immersive System Diagnostics
    text.push(Line::from(Span::styled(
        " ENVIRONMENT METRICS:",
        Style::default()
            .fg(COLOR_NEON_CYAN)
            .add_modifier(Modifier::BOLD),
    )));
    text.push(Line::from(format!(
        "  SECTOR DEPTH : {}",
        app.state.floor_number
    )));
    text.push(Line::from(format!(
        "  THREAT LEVEL : {}",
        if app.state.floor_number > 3 {
            "CRITICAL"
        } else {
            "ELEVATED"
        }
    )));
    text.push(Line::from("  LLM ENGINE   : Q4_K_M (ACTIVE)"));

    text.push(Line::from(""));

    // Directive
    text.push(Line::from(Span::styled(
        " ACTIVE DIRECTIVE:",
        Style::default()
            .fg(COLOR_NEON_CYAN)
            .add_modifier(Modifier::BOLD),
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
    text.push(Line::from("")); // Extra padding
    text.push(Line::from(Span::styled(
        "  [?, H, h] OPEN SYSTEM MANUAL",
        Style::default()
            .fg(COLOR_NEON_MAGENTA)
            .add_modifier(Modifier::RAPID_BLINK),
    )));

    let block = Paragraph::new(text)
        .block(create_sci_fi_block("DATABANKS"))
        .wrap(Wrap { trim: true });

    f.render_widget(block, area);
}

fn render_log(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = Vec::new();

    // Dynamically calculate how many messages can fit inside the box (height minus 2 for borders)
    let display_count = area.height.saturating_sub(2) as usize;
    let start_idx = app.log_messages.len().saturating_sub(display_count);

    for msg in &app.log_messages[start_idx..] {
        text.push(Line::from(Span::styled(
            format!(" {}", msg),
            Style::default().fg(Color::LightCyan),
        )));
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
        text.push(Line::from("Press 'U' during combat to extract."));
    }
    let block = Paragraph::new(text).block(
        Block::default()
            .title(" CACHE ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_NEON_MAGENTA)),
    );
    let popup_area = centered_rect(40, 40, area);
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

    // 1. Conversation History
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

    // 2. Input Box or Loading Indicator
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
            Style::default()
                .fg(COLOR_NEON_CYAN)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "TACTICAL LEGEND",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  @ Player      Ω Hostile"),
        Line::from("  ✦ Loot/Key    ▣ Locked Door"),
        Line::from("  ▼ Descent     · Floor / Open"),
        Line::from("  N NPC         T Puzzle Terminal"),
        Line::from("  ▲ Hazard"),
        Line::from(""),
        Line::from(Span::styled(
            "MOVEMENT & COMBAT",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("[W, A, S, D] : Navigate grid. Walk into enemies to strike."),
        Line::from("[E]          : Evade. Use when enemy flashes yellow to dodge."),
        Line::from("[Space]      : Wait one time-tick. Regain energy."),
        Line::from(""),
        Line::from(Span::styled(
            "WORLD INTERACTION",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("[F]          : Interact. Open doors, talk to NPCs, or access Terminals."),
        Line::from("[G]          : Grab item. Stand over ✦ to extract to inventory."),
        Line::from("[>]          : Descend stairs. Must be standing on ▼."),
        Line::from(""),
        Line::from(Span::styled(
            "SYSTEM PROTOCOLS",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("[I]          : Open Inventory Cache."),
        Line::from("[U]          : Consume top item in inventory (Heal)."),
        Line::from("[T]          : Toggle Display Theme (Cyberpunk / Monochrome)."),
        Line::from("[ESC]        : Pause system operations."),
        Line::from(""),
        Line::from(Span::styled(
            "PROGRESSION",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("• Defeating enemies grants XP. Leveling up fully heals and increases damage."),
        Line::from("• Reach Floor 5 to confront the Nexus Mind."),
        Line::from(""),
        Line::from(Span::styled(
            "Press [ESC] to close manual.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Paragraph::new(text)
        .block(
            Block::default()
                .title(" SYSTEM MANUAL ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(COLOR_NEON_CYAN)),
        )
        .alignment(ratatui::layout::Alignment::Left);

    // Increased popup dimensions to comfortably fit Phase 6/7 additions
    let popup_area = centered_rect(70, 90, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
}

fn render_puzzle_menu(
    f: &mut Frame,
    puzzle: &game_core::world::PuzzleType,
    state: &game_core::puzzle::PuzzleState,
    area: Rect,
) {
    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area);

    let mut text = Vec::new();

    match state {
        game_core::puzzle::PuzzleState::TokenVault {
            fragments,
            selected,
            target,
        } => {
            text.push(Line::from(Span::styled(
                "OBJECTIVE: Construct the sequence accurately.",
                Style::default().fg(COLOR_NEON_CYAN),
            )));
            text.push(Line::from(format!("TARGET: {}", target)));
            text.push(Line::from(""));
            text.push(Line::from("AVAILABLE FRAGMENTS (Press 0-9 to toggle):"));
            for (i, frag) in fragments.iter().enumerate() {
                let prefix = if selected.contains(&i) { "[x]" } else { "[ ]" };
                text.push(Line::from(format!(
                    "{} {}: '{}' (ID: {})",
                    prefix, i, frag.0, frag.1
                )));
            }
            let current: String = selected.iter().map(|&i| fragments[i].0).collect();
            text.push(Line::from(""));
            text.push(Line::from(format!("CURRENT BUFFER: {}", current)));
        }
        game_core::puzzle::PuzzleState::QuantizationForge { current_bits } => {
            text.push(Line::from(Span::styled(
                "OBJECTIVE: Balance precision against memory constraints.",
                Style::default().fg(COLOR_NEON_CYAN),
            )));
            text.push(Line::from("Use UP/DOWN to adjust weight precision."));
            text.push(Line::from(""));
            let memory = (*current_bits as f32 / 4.0) * 16.0;
            let coherence = if *current_bits == 32 {
                100
            } else if *current_bits == 16 {
                99
            } else if *current_bits == 8 {
                98
            } else if *current_bits == 4 {
                95
            } else {
                12
            };
            text.push(Line::from(format!("PRECISION: {}-bit", current_bits)));
            text.push(Line::from(format!("VRAM REQUIRED: {} GB", memory)));
            text.push(Line::from(format!("MODEL COHERENCE: {}%", coherence)));
        }
        game_core::puzzle::PuzzleState::ContextCollapse {
            logs,
            selected,
            max_budget,
        } => {
            text.push(Line::from(Span::styled(
                "OBJECTIVE: Prune KV Cache to retain essential data within budget.",
                Style::default().fg(COLOR_NEON_CYAN),
            )));
            text.push(Line::from(format!("CONTEXT BUDGET: {} Tokens", max_budget)));
            text.push(Line::from(""));
            let mut current_cost = 0;
            for (i, log) in logs.iter().enumerate() {
                let prefix = if selected.contains(&i) {
                    current_cost += log.1;
                    "[x]"
                } else {
                    "[ ]"
                };
                text.push(Line::from(format!(
                    "{} {}: {} ({}T)",
                    prefix, i, log.0, log.1
                )));
            }
            let color = if current_cost > *max_budget {
                Color::Red
            } else {
                COLOR_HUD_GREEN
            };
            text.push(Line::from(""));
            text.push(Line::from(Span::styled(
                format!("CURRENT USAGE: {}/{}", current_cost, max_budget),
                Style::default().fg(color),
            )));
        }
        game_core::puzzle::PuzzleState::InferenceReactor {
            stages,
            cursor,
            selected,
        } => {
            text.push(Line::from(Span::styled(
                "OBJECTIVE: Realign the forward pass pipeline.",
                Style::default().fg(COLOR_NEON_CYAN),
            )));
            text.push(Line::from("Use W/S to move cursor. SPACE to select/swap."));
            text.push(Line::from(""));
            for (i, stage) in stages.iter().enumerate() {
                let is_cursor = *cursor == i;
                let is_selected = *selected == Some(i);

                let style = if is_selected {
                    Style::default()
                        .fg(COLOR_NEON_MAGENTA)
                        .add_modifier(Modifier::BOLD)
                } else if is_cursor {
                    Style::default()
                        .fg(COLOR_NEON_CYAN)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(Color::White)
                };

                text.push(Line::from(Span::styled(format!("  {}  ", stage), style)));
            }
        }
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "[ENTER] Compile & Submit  [ESC] Abort",
        Style::default().fg(Color::DarkGray),
    )));

    let title = format!(" {:?} ", puzzle);
    let block = Paragraph::new(text).block(create_sci_fi_block(&title));
    f.render_widget(block, popup_area);
}

fn render_puzzle_solved_menu(f: &mut Frame, explanation: &str, area: Rect) {
    let popup_area = centered_rect(60, 40, area); // Make it wide enough for text wrapping
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
        .wrap(Wrap { trim: true }) // Automatically wraps the long explanation paragraph
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(block, popup_area);
}
