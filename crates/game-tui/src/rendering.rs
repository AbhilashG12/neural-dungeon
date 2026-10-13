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
        UiMode::Playing => {}
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
                let symbol = npc.name.chars().next().unwrap_or('N').to_string();
                row.push(Span::styled(symbol, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));            } else if let Some(enemy) = floor.get_enemy_at(x, y) {
                if matches!(
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
                }
            } else {
                match floor.get_tile(x, y).unwrap_or(Tile::Wall) {
                    Tile::Wall => row.push(Span::styled("█", Style::default().fg(COLOR_TECH_GRAY))),
                    Tile::Floor => {
                        row.push(Span::styled("·", Style::default().fg(Color::DarkGray)))
                    }
                    Tile::Door { is_open, .. } => {
                        if is_open {
                            row.push(Span::styled("·", Style::default().fg(Color::DarkGray))); // Open door looks like floor
                        } else {
                            row.push(Span::styled("▣", Style::default().fg(Color::Yellow))); // Closed door
                        }
                    }
                    Tile::StairsDown => {
                        row.push(Span::styled("▼", Style::default().fg(COLOR_NEON_MAGENTA)))
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
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top padding
            Constraint::Length(2), // HP Gauge
            Constraint::Length(2), // Energy Gauge
            Constraint::Length(1), // Weapon Text
        ])
        .split(inner_area);
    let center_80_percent = |r: Rect| -> Rect {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(10), Constraint::Percentage(80), Constraint::Percentage(10)])
            .split(r)[1]
    };

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
    let weapon_text = Paragraph::new(Span::styled("WEAPON: Plasma Blade", Style::default().fg(Color::DarkGray)))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(weapon_text, vertical_chunks[3]);
}


fn render_info(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = vec![];
    
    // Immersive System Diagnostics
    text.push(Line::from(Span::styled(" ENVIRONMENT METRICS:", Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD))));
    text.push(Line::from(format!("  SECTOR DEPTH : {}", app.state.floor_number)));
    text.push(Line::from(format!("  THREAT LEVEL : {}", if app.state.floor_number > 3 { "CRITICAL" } else { "ELEVATED" })));
    text.push(Line::from("  LLM ENGINE   : Q4_K_M (ACTIVE)"));
    
    text.push(Line::from(""));
    
    // Directive
    text.push(Line::from(Span::styled(" ACTIVE DIRECTIVE:", Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD))));
    for quest in &app.state.player.active_quests {
        let (icon, color) = if quest.completed { ("[x]", COLOR_HUD_GREEN) } else { ("[ ]", Color::Yellow) };
        text.push(Line::from(Span::styled(format!("  {} {}", icon, quest.title), Style::default().fg(color))));
    }
    
    text.push(Line::from(""));
    text.push(Line::from("")); // Extra padding
    text.push(Line::from(Span::styled("  [?, H, h] OPEN SYSTEM MANUAL", Style::default().fg(COLOR_NEON_MAGENTA).add_modifier(Modifier::RAPID_BLINK))));

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
        text.push(Line::from(Span::styled(format!(" {}", msg), Style::default().fg(Color::LightCyan))));
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
        Line::from(Span::styled("NEURAL DUNGEON OPERATING MANUAL", Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("TACTICAL LEGEND", Style::default().fg(Color::Yellow))),
        Line::from("  @ Player      Ω Hostile"),
        Line::from("  ✦ Loot/Key    ▣ Locked Door"),
        Line::from("  ▼ Descent     · Floor / Open"),
        Line::from("  D Dr. Aris    M Maint. Node"),
        Line::from("  S Security"),
        Line::from(""),
        Line::from(Span::styled("MOVEMENT & COMBAT", Style::default().fg(Color::Yellow))),
        Line::from("[W, A, S, D] : Navigate grid. Walk into enemies to strike."),
        Line::from("[E]          : Evade. Use when enemy flashes yellow to dodge."),
        Line::from("[Space]      : Wait one time-tick. Regain energy."),
        Line::from(""),
        Line::from(Span::styled("WORLD INTERACTION", Style::default().fg(Color::Yellow))),
        Line::from("[F]          : Interact. Open doors or talk to NPCs."),
        Line::from("[G]          : Grab item. Stand over ✦ to extract to inventory."),
        Line::from("[>]          : Descend stairs. Must be standing on ▼."),
        Line::from(""),
        Line::from(Span::styled("SYSTEM PROTOCOLS", Style::default().fg(Color::Yellow))),
        Line::from("[I]          : Open Inventory Cache."),
        Line::from("[U]          : Consume top item in inventory (Heal)."),
        Line::from("[ESC]        : Pause system operations."),
        Line::from(""),
        Line::from(Span::styled("Press [?] or [H] or [ESC] to close manual.", Style::default().fg(Color::DarkGray))),
    ];

    let block = Paragraph::new(text)
        .block(Block::default().title(" SYSTEM MANUAL ").borders(Borders::ALL).border_type(BorderType::Double).border_style(Style::default().fg(COLOR_NEON_CYAN)))
        .alignment(ratatui::layout::Alignment::Left);
    
    // Increased popup height to 75% to easily fit the new legend section
    let popup_area = centered_rect(65, 85, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
}
