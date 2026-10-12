use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use crate::app::{GameApp, UiMode};
use game_core::world::Tile;

pub fn render(f: &mut Frame, app: &GameApp) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75),
            Constraint::Percentage(25),
        ])
        .split(f.size());

    let sidebar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),      
            Constraint::Min(10),        
            Constraint::Length(8),     
        ])
        .split(main_chunks[1]);

    render_map(f, app, main_chunks[0]);
    render_stats(f, app, sidebar_chunks[0]);
    render_info(f, app, sidebar_chunks[1]);
    render_log(f, app, sidebar_chunks[2]);

    match app.ui_mode {
        UiMode::Paused => render_pause_menu(f, f.size()),
        UiMode::Inventory => render_inventory_menu(f, app, f.size()),
        UiMode::Playing => {}
    }
}

fn render_map(f: &mut Frame, app: &GameApp, area: Rect) {
    let floor = &app.state.current_floor;
    let mut map_lines = Vec::new();

    for y in 0..floor.height as i32 {
        let mut row = Vec::new();
        for x in 0..floor.width as i32 {
            if app.state.player.position == (x, y) {
                row.push(Span::styled("@", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
            } else if let Some(_) = floor.get_enemy_at(x, y) {
                row.push(Span::styled("E", Style::default().fg(Color::Red)));
            } else {
                match floor.get_tile(x, y).unwrap_or(Tile::Wall) {
                    Tile::Wall => row.push(Span::styled("#", Style::default().fg(Color::DarkGray))),
                    Tile::Floor => row.push(Span::styled(".", Style::default().fg(Color::Gray))),
                    Tile::Door { .. } => row.push(Span::styled("+", Style::default().fg(Color::Yellow))),
                    Tile::StairsDown => row.push(Span::styled(">", Style::default().fg(Color::Magenta))),
                }
            }
        }
        map_lines.push(Line::from(row));
    }

    let map_block = Paragraph::new(map_lines)
        .block(Block::default().title(" FLOOR 01: THE FORGOTTEN LAB ").borders(Borders::ALL));
    
    f.render_widget(map_block, area);
}

fn render_stats(f: &mut Frame, app: &GameApp, area: Rect) {
    let p = &app.state.player;
    let text = vec![
        Line::from(format!("HP:     {}/{}", p.hp, p.max_hp)),
        Line::from(format!("ENERGY: {}/100", p.energy)),
        Line::from(""),
        Line::from("WEAPON: Plasma Blade"),
    ];

    let block = Paragraph::new(text)
        .block(Block::default().title(" STATUS ").borders(Borders::ALL));
    f.render_widget(block, area);
}

fn render_info(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = vec![Line::from("OBJECTIVES:")];
    
    for quest in &app.state.player.active_quests {
        let status = if quest.completed { "[x]" } else { "[ ]" };
        text.push(Line::from(format!("{} {}", status, quest.title)));
    }
    
    text.push(Line::from(""));
    text.push(Line::from("CONTROLS:"));
    text.push(Line::from("[WASD] Move   [>] Descend"));
    text.push(Line::from("[I] Inventory [ESC] Menu"));

    let block = Paragraph::new(text)
        .block(Block::default().title(" DATA ").borders(Borders::ALL));
    f.render_widget(block, area);
}

fn render_log(f: &mut Frame, app: &GameApp, area: Rect) {
    let mut text = Vec::new();
    for msg in &app.log_messages {
        text.push(Line::from(msg.clone()));
    }

    let block = Paragraph::new(text)
        .block(Block::default().title(" LOG ").borders(Borders::ALL))
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
    let block = Paragraph::new("PAUSED\n\n[ESC] Resume\n[Q] Quit Game")
        .block(Block::default().title(" SYSTEM HALTED ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    let area = centered_rect(30, 20, area);
    f.render_widget(Clear, area); 
    f.render_widget(block, area);
}

fn render_inventory_menu(f: &mut Frame, _app: &GameApp, area: Rect) {
    let block = Paragraph::new("Inventory is empty.\n\n[ESC] Close")
        .block(Block::default().title(" INVENTORY ").borders(Borders::ALL));

    let area = centered_rect(40, 40, area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
}
