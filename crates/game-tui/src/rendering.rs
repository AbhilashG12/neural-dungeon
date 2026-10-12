use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Clear, Paragraph, Wrap, Gauge},
    Frame,
};
use crate::app::{GameApp, UiMode};
use game_core::world::Tile;

// UI Theme Colors (Cyberpunk/Sci-Fi Palette)
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
            Constraint::Length(10), // Stats Gauges
            Constraint::Min(12),    // Objectives/Controls
            Constraint::Length(10), // Combat Log
        ])
        .split(main_chunks[1]);

    render_map(f, app, main_chunks[0]);
    render_stats(f, app, sidebar_chunks[0]);
    render_info(f, app, sidebar_chunks[1]);
    render_log(f, app, sidebar_chunks[2]);

    match app.ui_mode {
        UiMode::Paused => render_pause_menu(f, f.size()),
        UiMode::Inventory => render_inventory_menu(f, app, f.size()),
        UiMode::GameOver => render_game_over_menu(f, f.size()),
        UiMode::Playing => {}
    }
}

fn create_sci_fi_block(title: &str) -> Block<'static> {
    Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(COLOR_NEON_CYAN))
        .title_style(Style::default().fg(COLOR_NEON_CYAN).add_modifier(Modifier::BOLD))
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
                let color = if app.state.player.evading { COLOR_NEON_MAGENTA } else { COLOR_NEON_CYAN };
                row.push(Span::styled("@", Style::default().fg(color).add_modifier(Modifier::BOLD)));
            } else if let Some(enemy) = floor.get_enemy_at(x, y) {
                if matches!(enemy.state, game_core::world::EnemyState::Telegraphing{..}) {
                    row.push(Span::styled("Ω", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                } else {
                    row.push(Span::styled("Ω", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
                }
            } else if floor.items.iter().any(|i| i.position == (x, y)) {
                row.push(Span::styled("✦", Style::default().fg(COLOR_HUD_GREEN).add_modifier(Modifier::RAPID_BLINK)));
            } else {
                match floor.get_tile(x, y).unwrap_or(Tile::Wall) {
                    Tile::Wall => row.push(Span::styled("█", Style::default().fg(COLOR_TECH_GRAY))),
                    Tile::Floor => row.push(Span::styled("·", Style::default().fg(Color::DarkGray))),
                    Tile::Door { is_open, .. } => {
                        if is_open {
                            row.push(Span::styled("·", Style::default().fg(Color::DarkGray))); // Open door looks like floor
                        } else {
                            row.push(Span::styled("▣", Style::default().fg(Color::Yellow))); // Closed door
                        }
                    }
                    Tile::StairsDown => row.push(Span::styled("▼", Style::default().fg(COLOR_NEON_MAGENTA))),
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
    
    f.render_widget(create_sci_fi_block("NEURAL LINK"), area);
    
    let inner_area = area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 2 });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Length(2)])
        .split(inner_area);

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

    f.render_widget(hp_gauge, chunks[0]);
    f.render_widget(energy_gauge, chunks[1]);
    f.render_widget(Paragraph::new("WEAPON: Plasma Blade").style(Style::default().fg(Color::DarkGray)), chunks[2]);
}

fn render_info(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = vec![Line::from(Span::styled("ACTIVE DIRECTIVES:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)))];
    
    for quest in &app.state.player.active_quests {
        let (icon, color) = if quest.completed { ("[COMPLETED]", COLOR_HUD_GREEN) } else { ("[PENDING]", Color::Yellow) };
        text.push(Line::from(Span::styled(format!("{} {}", icon, quest.title), Style::default().fg(color))));
    }
    
    text.push(Line::from(""));
    text.push(Line::from(Span::styled("TERMINAL CONTROLS:", Style::default().fg(Color::White).add_modifier(Modifier::BOLD))));
    text.push(Line::from("[W,A,S,D] Navigate grid"));
    text.push(Line::from("[F] Override doors"));
    text.push(Line::from("[G] Extract loot"));
    text.push(Line::from("[E] Evasive maneuvers"));
    text.push(Line::from("[U] Consume top item"));
    text.push(Line::from("[I] Access inventory"));
    text.push(Line::from("[>] Descend sector"));

    let block = Paragraph::new(text).block(create_sci_fi_block("DATABANKS"));
    f.render_widget(block, area);
}

fn render_log(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = Vec::new();
    for msg in &app.log_messages {
        text.push(Line::from(Span::styled(msg.clone(), Style::default().fg(Color::LightCyan))));
    }
    let block = Paragraph::new(text).block(create_sci_fi_block("TELEMETRY LOG")).wrap(Wrap { trim: true });
    f.render_widget(block, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage((100 - percent_y) / 2), Constraint::Percentage(percent_y), Constraint::Percentage((100 - percent_y) / 2)])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage((100 - percent_x) / 2), Constraint::Percentage(percent_x), Constraint::Percentage((100 - percent_x) / 2)])
        .split(popup_layout[1])[1]
}

fn render_pause_menu(f: &mut Frame, area: Rect) {
    let block = Paragraph::new("\n\n[ESC] Resume Protocol\n[Q] Terminate Session")
        .block(Block::default().title(" SYSTEM HALTED ").borders(Borders::ALL).border_type(BorderType::Double))
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
    let block = Paragraph::new(text).block(Block::default().title(" CACHE ").borders(Borders::ALL).border_style(Style::default().fg(COLOR_NEON_MAGENTA)));
    let popup_area = centered_rect(40, 40, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
}

fn render_game_over_menu(f: &mut Frame, area: Rect) {
    let text = vec![Line::from(""), Line::from("[R] Initialize New Clone"), Line::from("[Q] Terminate")];
    let block = Paragraph::new(text)
        .block(Block::default().title(" CRITICAL SYSTEM FAILURE ").borders(Borders::ALL).border_type(BorderType::Thick))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red).bg(Color::Black));
    let popup_area = centered_rect(30, 20, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);
}
