use crate::app::{App, Tab};
use chrono::{DateTime, Local, TimeZone, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // now playing
            Constraint::Length(3), // tabs
            Constraint::Min(1),    // list
            Constraint::Length(1), // status bar
        ])
        .split(area);

    draw_header(f, app, chunks[0]);
    draw_now_playing(f, app, chunks[1]);
    draw_tabs(f, app, chunks[2]);
    draw_list(f, app, chunks[3]);
    draw_status(f, app, chunks[4]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.cfg.colors;
    let total = app.total.map(|n| format!("{n} listens")).unwrap_or_else(|| "—".into());
    let title = Line::from(vec![
        Span::styled(" mstatui ", Style::default().fg(p.bg).bg(p.accent).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(&app.cfg.username, Style::default().fg(p.accent).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(format!("· {total}"), Style::default().fg(p.muted)),
        Span::raw("  "),
        Span::styled(format!("· range: {}", app.range.as_str()), Style::default().fg(p.muted)),
    ]);
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(p.muted));
    f.render_widget(Paragraph::new(title).block(block), area);
}

fn draw_now_playing(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.cfg.colors;
    let line = match &app.now_playing {
        Some(np) => Line::from(vec![
            Span::styled(" ▸ ", Style::default().fg(p.positive)),
            Span::styled(&np.track_metadata.artist_name, Style::default().fg(p.accent).add_modifier(Modifier::BOLD)),
            Span::raw(" — "),
            Span::styled(&np.track_metadata.track_name, Style::default().fg(p.fg)),
            Span::styled(
                np.track_metadata.release_name.as_deref().map(|r| format!("  · {r}")).unwrap_or_default(),
                Style::default().fg(p.muted),
            ),
        ]),
        None => Line::from(Span::styled(" ▸ nothing playing", Style::default().fg(p.muted))),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Now Playing ")
        .border_style(Style::default().fg(p.muted));
    f.render_widget(Paragraph::new(line).block(block), area);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.cfg.colors;
    let titles: Vec<Line> = Tab::ALL.iter().map(|t| Line::from(t.label())).collect();
    let idx = Tab::ALL.iter().position(|t| *t == app.tab).unwrap_or(0);
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(p.muted)))
        .style(Style::default().fg(p.muted))
        .highlight_style(Style::default().fg(p.accent).add_modifier(Modifier::BOLD))
        .select(idx);
    f.render_widget(tabs, area);
}

fn draw_list(f: &mut Frame, app: &mut App, area: Rect) {
    let p = &app.cfg.colors;
    let items: Vec<ListItem> = match app.tab {
        Tab::Recent => app.recent.iter().map(|l| recent_item(l, p)).collect(),
        Tab::Artists => artists_items(&app.artists, p),
        Tab::Tracks => tracks_items(&app.tracks, p),
        Tab::Releases => releases_items(&app.releases, p),
    };

    let title = format!(" {} ", app.tab.label());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(p.muted));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(p.muted).fg(p.fg).add_modifier(Modifier::BOLD))
        .highlight_symbol(" ▸ ");

    let mut state = ListState::default();
    if app.list_len() > 0 {
        state.select(Some(app.selected.min(app.list_len() - 1)));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let p = &app.cfg.colors;
    let k = &app.cfg.keys;
    let help = format!(
        "  {} quit · {}/{} tabs · {}/{} move · {}/{} top/bot · {} refresh · {} range · ⏎ open  ",
        k.quit, k.prev_tab, k.next_tab, k.up, k.down, k.top, k.bottom, k.refresh, k.toggle_range
    );
    let badge_color = if app.refreshing { p.warning } else { p.positive };
    let badge_text = if app.refreshing { " ⟳ " } else { " ● " };
    let stale = if app.fetched_at > 0 {
        format!(" data: {}", humanize(app.fetched_at))
    } else {
        " data: —".into()
    };
    let line = Line::from(vec![
        Span::styled(badge_text, Style::default().fg(p.bg).bg(badge_color).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {} ", app.status), Style::default().fg(p.fg).bg(p.muted)),
        Span::styled(stale, Style::default().fg(p.muted)),
        Span::styled(help, Style::default().fg(p.muted)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn recent_item<'a>(l: &'a crate::api::Listen, p: &crate::config::Palette) -> ListItem<'a> {
    let when = humanize(l.listened_at);
    let line = Line::from(vec![
        Span::styled(&l.track_metadata.artist_name, Style::default().fg(p.accent)),
        Span::raw(" — "),
        Span::styled(&l.track_metadata.track_name, Style::default().fg(p.fg)),
        Span::styled(format!("   {when}"), Style::default().fg(p.muted)),
    ]);
    ListItem::new(line)
}

fn artists_items<'a>(v: &'a [crate::api::TopArtist], p: &crate::config::Palette) -> Vec<ListItem<'a>> {
    let max = v.iter().map(|a| a.listen_count).max().unwrap_or(1).max(1);
    v.iter()
        .enumerate()
        .map(|(i, a)| {
            let bar = bar(a.listen_count, max, 24);
            let line = Line::from(vec![
                Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(p.muted)),
                Span::styled(format!("{:<32}", truncate(&a.artist_name, 32)), Style::default().fg(p.accent)),
                Span::styled(bar, Style::default().fg(p.positive)),
                Span::styled(format!(" {}", a.listen_count), Style::default().fg(p.fg)),
            ]);
            ListItem::new(line)
        })
        .collect()
}

fn tracks_items<'a>(v: &'a [crate::api::TopRecording], p: &crate::config::Palette) -> Vec<ListItem<'a>> {
    let max = v.iter().map(|t| t.listen_count).max().unwrap_or(1).max(1);
    v.iter()
        .enumerate()
        .map(|(i, t)| {
            let bar = bar(t.listen_count, max, 16);
            let line = Line::from(vec![
                Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(p.muted)),
                Span::styled(format!("{:<24}", truncate(&t.artist_name, 24)), Style::default().fg(p.accent)),
                Span::styled(format!("{:<32}", truncate(&t.track_name, 32)), Style::default().fg(p.fg)),
                Span::styled(bar, Style::default().fg(p.positive)),
                Span::styled(format!(" {}", t.listen_count), Style::default().fg(p.fg)),
            ]);
            ListItem::new(line)
        })
        .collect()
}

fn releases_items<'a>(v: &'a [crate::api::TopRelease], p: &crate::config::Palette) -> Vec<ListItem<'a>> {
    let max = v.iter().map(|r| r.listen_count).max().unwrap_or(1).max(1);
    v.iter()
        .enumerate()
        .map(|(i, r)| {
            let bar = bar(r.listen_count, max, 16);
            let line = Line::from(vec![
                Span::styled(format!("{:>2}. ", i + 1), Style::default().fg(p.muted)),
                Span::styled(format!("{:<24}", truncate(&r.artist_name, 24)), Style::default().fg(p.accent)),
                Span::styled(format!("{:<32}", truncate(&r.release_name, 32)), Style::default().fg(p.fg)),
                Span::styled(bar, Style::default().fg(p.positive)),
                Span::styled(format!(" {}", r.listen_count), Style::default().fg(p.fg)),
            ]);
            ListItem::new(line)
        })
        .collect()
}

fn bar(value: u64, max: u64, width: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let filled = ((value as f64 / max as f64) * width as f64).round() as usize;
    let mut s = String::with_capacity(width);
    for _ in 0..filled {
        s.push('█');
    }
    for _ in filled..width {
        s.push('·');
    }
    s
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn humanize(ts: i64) -> String {
    if ts <= 0 {
        return "—".into();
    }
    let then: DateTime<Utc> = match Utc.timestamp_opt(ts, 0) {
        chrono::LocalResult::Single(t) => t,
        _ => return "—".into(),
    };
    let now = Utc::now();
    let delta = now - then;
    let secs = delta.num_seconds();
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3600)
    } else if secs < 7 * 86_400 {
        format!("{}d ago", secs / 86_400)
    } else {
        then.with_timezone(&Local).format("%Y-%m-%d").to_string()
    }
}
